use prometheus_client::PrometheusClient;
use chrono::Utc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let prometheus_url = "http://localhost:9090";
    let client = PrometheusClient::new(prometheus_url);
    
    // 監視対象のメトリクス
    let metrics = vec![
        r#"{job="rust-app", __name__="total_tx_bytes_rate"}"#,
        r#"{job="rust-app", __name__="total_rx_bytes_rate"}"#,
    ];
    
    println!("メトリクス監視を開始します (Ctrl+C で停止)...\n");
    
    loop {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        println!("=== {} ===", timestamp);
        
        for metric_query in &metrics {
            match client.query(metric_query).await {
                Ok(response) => {
                    if response.data.result.is_empty() {
                        println!("{}の結果が見つかりませんでした", metric_query);
                    } else {
                        for result in &response.data.result {
                            if let Some(metric_name) = result.metric.get("__name__") {
                                if let Some(value) = &result.value {
                                    println!("{}: {}", metric_name, value.1);
                                } else {
                                    println!("{}: 値なし", metric_name);
                                }
                            }
                        }
                    }
                }
                Err(e) => println!("クエリエラー ({}): {}", metric_query, e),
            }
        }
        
        println!(); // 空行で区切る
        
        // 0.1秒待機
        sleep(Duration::from_millis(1000)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_prometheus_client_creation() {
        let client = PrometheusClient::new("http://localhost:9090");
        // 基本的なテスト
        assert!(true);
    }
}
