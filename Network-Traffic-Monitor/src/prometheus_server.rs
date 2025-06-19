use std::time::Duration;
use std::sync::Arc;
use std::net::SocketAddr;
use prometheus::{Counter, Gauge, GaugeVec, Registry, TextEncoder};
use prometheus::core::Collector;
use hyper::{Request, Response, Method, StatusCode};
use hyper::service::service_fn;
use hyper::server::conn::http1;
use hyper::body::Bytes;
use http_body_util::Full;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::time;
use rand::Rng;
use std::sync::Mutex;
use crate::stats::IpStatsMap;

// グローバルネットワークメトリクス（capture.rsから共有）
static NETWORK_METRICS: std::sync::OnceLock<Arc<Mutex<crate::capture::NetworkMetrics>>> = std::sync::OnceLock::new();
static IP_STATS: std::sync::OnceLock<IpStatsMap> = std::sync::OnceLock::new();

pub fn set_network_metrics(metrics: Arc<Mutex<crate::capture::NetworkMetrics>>) {
    let _ = NETWORK_METRICS.set(metrics);
}

pub fn set_ip_stats(stats: IpStatsMap) {
    let _ = IP_STATS.set(stats);
}

// メトリクス構造体
#[derive(Clone)]
struct AppMetrics {
    registry: Registry,
    active_connections: Gauge,
    cpu_usage: Gauge,
    memory_usage: Gauge,
}

impl AppMetrics {
    fn new() -> Self {
        let registry = Registry::new();
        
        let active_connections = Gauge::new("active_connections", "Number of active connections").unwrap();
        let cpu_usage = Gauge::new("cpu_usage_percent", "CPU usage percentage").unwrap();
        let memory_usage = Gauge::new("memory_usage_bytes", "Memory usage in bytes").unwrap();

        // レジストリにメトリクスを登録
        registry.register(Box::new(active_connections.clone())).unwrap();
        registry.register(Box::new(cpu_usage.clone())).unwrap();
        registry.register(Box::new(memory_usage.clone())).unwrap();

        AppMetrics {
            registry,
            active_connections,
            cpu_usage,
            memory_usage,
        }
    }

    fn export(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode_to_string(&metric_families).unwrap()
    }
}

// HTTPハンドラー
async fn handle_request(
    req: Request<hyper::body::Incoming>,
    metrics: Arc<AppMetrics>,
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    metrics.active_connections.inc();

    let response = match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            Response::builder()
                .status(StatusCode::OK)
                .body(Full::new(Bytes::from("Hello, Prometheus!")))
                .unwrap()
        }
        (&Method::GET, "/metrics") => {
            // アプリケーションメトリクスを取得
            let app_metrics_output = metrics.export();
            
            // ネットワークメトリクスを取得（利用可能な場合）
            let network_metrics_output = if let Some(network_metrics) = NETWORK_METRICS.get() {
                if let Ok(network_metrics) = network_metrics.lock() {
                    network_metrics.export()
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            // IP統計情報を取得
            let ip_stats_output = if let Some(ip_stats) = IP_STATS.get() {
                if let Ok(ip_stats) = ip_stats.lock() {
                    let encoder = TextEncoder::new();

                    let local_ip_tx_bytes = GaugeVec::new(prometheus::Opts::new("local_ip_tx_bytes_per_sec", "Transmit bytes per second for each local IP"), &["local_ip"]).unwrap();
                    let local_ip_rx_bytes = GaugeVec::new(prometheus::Opts::new("local_ip_rx_bytes_per_sec", "Receive bytes per second for each local IP"), &["local_ip"]).unwrap();
                    let local_ip_tx_packets = GaugeVec::new(prometheus::Opts::new("local_ip_tx_packets_per_sec", "Transmit packets per second for each local IP"), &["local_ip"]).unwrap();
                    let local_ip_rx_packets = GaugeVec::new(prometheus::Opts::new("local_ip_rx_packets_per_sec", "Receive packets per second for each local IP"), &["local_ip"]).unwrap();

                    for (ip, stats) in ip_stats.iter() {
                        let ip_str = ip.to_string();
                        local_ip_tx_bytes.with_label_values(&[&ip_str]).set(stats.tx_bytes_per_sec as f64);
                        local_ip_rx_bytes.with_label_values(&[&ip_str]).set(stats.rx_bytes_per_sec as f64);
                        local_ip_tx_packets.with_label_values(&[&ip_str]).set(stats.tx_packets_per_sec as f64);
                        local_ip_rx_packets.with_label_values(&[&ip_str]).set(stats.rx_packets_per_sec as f64);
                    }
                    
                    let mut metric_families = vec![];
                    metric_families.extend(local_ip_tx_bytes.collect());
                    metric_families.extend(local_ip_rx_bytes.collect());
                    metric_families.extend(local_ip_tx_packets.collect());
                    metric_families.extend(local_ip_rx_packets.collect());

                    encoder.encode_to_string(&metric_families).unwrap()
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            // メトリクスを結合
            let mut combined_metrics = app_metrics_output;
            combined_metrics.push_str(&network_metrics_output);
            combined_metrics.push_str(&ip_stats_output);

            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/plain; version=0.0.4")
                .body(Full::new(Bytes::from(combined_metrics)))
                .unwrap()
        }
        (&Method::GET, "/health") => {
            Response::builder()
                .status(StatusCode::OK)
                .body(Full::new(Bytes::from("OK")))
                .unwrap()
        }
        _ => {
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from("Not Found")))
                .unwrap()
        }
    };

    metrics.active_connections.dec();

    Ok(response)
}

// メトリクス更新タスク（サンプルデータを生成）
async fn update_metrics_task(metrics: Arc<AppMetrics>) {
    let mut interval = time::interval(Duration::from_secs(5));

    loop {
        interval.tick().await;
        
        let mut rng = rand::thread_rng();
        
        // ランダムなCPU使用率 (0-100%)
        let cpu_percent = rng.gen_range(10.0..80.0);
        metrics.cpu_usage.set(cpu_percent);
        
        // ランダムなメモリ使用量 (100MB - 500MB)
        let memory_bytes = rng.gen_range(100_000_000..500_000_000) as f64;
        metrics.memory_usage.set(memory_bytes);
    }
}

// ライブラリ関数として公開するstart_prometheus_server
pub async fn start_prometheus_server(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Starting Prometheus Rust App on port {}...", port);
    
    let metrics = Arc::new(AppMetrics::new());
    
    // メトリクス更新タスクを開始
    let metrics_clone = metrics.clone();
    tokio::spawn(async move {
        update_metrics_task(metrics_clone).await;
    });

    // HTTPサーバーを設定
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;

    println!("Server running on http://0.0.0.0:{}", port);
    println!("Metrics available at http://0.0.0.0:{}/metrics", port);
    println!("Health check available at http://0.0.0.0:{}/health", port);

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let metrics = metrics.clone();
        
        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(move |req| {
                    handle_request(req, metrics.clone())
                }))
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}


