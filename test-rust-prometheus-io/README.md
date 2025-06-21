# Prometheus Rust Client

このプロジェクトは、Prometheusサーバからメトリクスデータを取得するRustクライアントライブラリです。

## 機能

- **即時クエリ**: 現在の値を取得
- **範囲クエリ**: 時系列データを取得
- **メトリクス一覧**: 利用可能なメトリクス名を取得
- **ラベル値**: 特定のメトリクスのラベル値を取得

## 前提条件

- Rust 1.70以上
- Prometheusサーバが`localhost:9090`で動作していること

## 使用方法

### プロジェクトのビルドと実行

```bash
# 依存関係をインストールしてビルド
cargo build

# 実行
cargo run
```

### テストの実行

```bash
cargo test
```

## コード例

### 基本的な使用方法

```rust
use prometheus_client::PrometheusClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = PrometheusClient::new("http://localhost:9090");
    
    // 即時クエリ
    let response = client.query("up").await?;
    println!("結果: {:?}", response);
    
    // 範囲クエリ
    let now = chrono::Utc::now();
    let one_hour_ago = now - chrono::Duration::hours(1);
    let range_response = client.query_range("up", one_hour_ago, now, "5m").await?;
    println!("範囲クエリ結果: {:?}", range_response);
    
    Ok(())
}
```

### よく使われるPrometheusクエリ

- `up`: サービスの稼働状態
- `prometheus_build_info`: Prometheusのビルド情報
- `rate(http_requests_total[5m])`: HTTPリクエストのレート
- `cpu_usage_percent`: CPU使用率
- `memory_usage_bytes`: メモリ使用量

## API リファレンス

### PrometheusClient

#### new(prometheus_url: &str) -> Self
新しいPrometheusクライアントを作成します。

#### async fn query(&self, query: &str) -> Result<PrometheusResponse, Error>
即時クエリを実行して現在の値を取得します。

#### async fn query_range(&self, query: &str, start: DateTime<Utc>, end: DateTime<Utc>, step: &str) -> Result<PrometheusResponse, Error>
指定した時間範囲の時系列データを取得します。

#### async fn get_label_names(&self) -> Result<Vec<String>, Error>
利用可能なメトリクス名のリストを取得します。

#### async fn get_label_values(&self, label: &str) -> Result<Vec<String>, Error>
特定のラベルの値のリストを取得します。

## 設定

環境変数でPrometheusサーバのURLを変更できます：

```bash
export PROMETHEUS_URL=http://your-prometheus-server:9090
cargo run
```

## トラブルシューティング

### Prometheusサーバに接続できない場合

1. Prometheusサーバが起動していることを確認
2. ポート9090が利用可能であることを確認
3. ファイアウォール設定を確認

### メトリクスが見つからない場合

1. Prometheusの設定でスクレイプターゲットが正しく設定されているか確認
2. `/metrics`エンドポイントが利用可能であることを確認
