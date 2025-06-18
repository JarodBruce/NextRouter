use std::time::Duration;
use std::sync::Arc;
use std::net::SocketAddr;
use prometheus::{Counter, Gauge, Histogram, Registry, TextEncoder};
use hyper::{Request, Response, Method, StatusCode};
use hyper::service::service_fn;
use hyper::server::conn::http1;
use hyper::body::Bytes;
use http_body_util::Full;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::time;
use rand::Rng;

// メトリクス構造体
#[derive(Clone)]
struct AppMetrics {
    registry: Registry,
    requests_total: Counter,
    active_connections: Gauge,
    request_duration: Histogram,
    cpu_usage: Gauge,
    memory_usage: Gauge,
}

impl AppMetrics {
    fn new() -> Self {
        let registry = Registry::new();
        
        let requests_total = Counter::new("http_requests_total", "Total HTTP requests").unwrap();
        let active_connections = Gauge::new("active_connections", "Number of active connections").unwrap();
        let request_duration = Histogram::with_opts(
            prometheus::HistogramOpts::new("request_duration_seconds", "Request duration in seconds")
                .buckets(vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0])
        ).unwrap();
        let cpu_usage = Gauge::new("cpu_usage_percent", "CPU usage percentage").unwrap();
        let memory_usage = Gauge::new("memory_usage_bytes", "Memory usage in bytes").unwrap();

        // レジストリにメトリクスを登録
        registry.register(Box::new(requests_total.clone())).unwrap();
        registry.register(Box::new(active_connections.clone())).unwrap();
        registry.register(Box::new(request_duration.clone())).unwrap();
        registry.register(Box::new(cpu_usage.clone())).unwrap();
        registry.register(Box::new(memory_usage.clone())).unwrap();

        AppMetrics {
            registry,
            requests_total,
            active_connections,
            request_duration,
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
    let start_time = std::time::Instant::now();
    metrics.requests_total.inc();
    metrics.active_connections.inc();

    let response = match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            Response::builder()
                .status(StatusCode::OK)
                .body(Full::new(Bytes::from("Hello, Prometheus!")))
                .unwrap()
        }
        (&Method::GET, "/metrics") => {
            let metrics_output = metrics.export();
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/plain; version=0.0.4")
                .body(Full::new(Bytes::from(metrics_output)))
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

    let duration = start_time.elapsed().as_secs_f64();
    metrics.request_duration.observe(duration);
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

        println!("Updated metrics - CPU: {:.1}%, Memory: {:.1}MB", 
                cpu_percent, memory_bytes / 1_000_000.0);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Starting Prometheus Rust App on port 8080...");
    
    let metrics = Arc::new(AppMetrics::new());
    
    // メトリクス更新タスクを開始
    let metrics_clone = metrics.clone();
    tokio::spawn(async move {
        update_metrics_task(metrics_clone).await;
    });

    // HTTPサーバーを設定
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = TcpListener::bind(addr).await?;

    println!("Server running on http://0.0.0.0:8080");
    println!("Metrics available at http://0.0.0.0:8080/metrics");
    println!("Health check available at http://0.0.0.0:8080/health");

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
