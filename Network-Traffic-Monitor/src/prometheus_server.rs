use crate::stats::IpStatsMap;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use prometheus::{Registry, TextEncoder};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::net::TcpListener;

// グローバルネットワークメトリクス（capture.rsから共有）
static NETWORK_METRICS: std::sync::OnceLock<Arc<Mutex<crate::capture::NetworkMetrics>>> =
    std::sync::OnceLock::new();
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
}

impl AppMetrics {
    fn new() -> Self {
        let registry = Registry::new();

        AppMetrics { registry }
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
    let response = match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => Response::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::from("Hello, Prometheus!")))
            .unwrap(),
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
                    // IP統計が空でない場合のみメトリクスを生成
                    if !ip_stats.is_empty() {
                        let _encoder = TextEncoder::new();
                        // TODO: Implement IP stats to Prometheus metrics conversion
                        String::new()
                    } else {
                        String::new()
                    }
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
        (&Method::GET, "/health") => Response::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::from("OK")))
            .unwrap(),
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from("Not Found")))
            .unwrap(),
    };

    Ok(response)
}

// ライブラリ関数として公開するstart_prometheus_server
pub async fn start_prometheus_server(
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Starting Prometheus Rust App on port {}...", port);

    let metrics = Arc::new(AppMetrics::new());

    // HTTPサーバーを設定
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;

    println!("Server running on http://0.0.0.0:{}", port);
    println!("Metrics available at http://0.0.0.0:{}/metrics", port);
    println!("Health check available at http://0.0.0.0:{}/health", port);

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, _)) => {
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
                    Err(e) => {
                        eprintln!("Failed to accept connection: {}", e);
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("Prometheus server received shutdown signal");
                break;
            }
        }
    }

    println!("Prometheus server stopped");

    Ok(())
}
