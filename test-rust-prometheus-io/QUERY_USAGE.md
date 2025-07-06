# カスタムPrometheusクエリの使用方法

このプログラムは、指定されたPrometheusクエリ:
```
{job="rust-app", __name__!="memory_usage_bytes",__name__!="cpu_usage_percent",__name__!="scrape_samples_scraped",__name__!="scrape_samples_post_metric_relabeling",__name__!="scrape_duration_seconds",__name__!="scrape_series_added",__name__!="up",__name__!="active_connections"}
```

を使用してrust-appジョブから特定のメトリクスを除外してデータを取得します。

## クエリの説明

- `job="rust-app"`: rust-appジョブのメトリクスのみを対象
- `__name__!="メトリクス名"`: 指定したメトリクス名を除外

### 除外されるメトリクス
- `memory_usage_bytes`: メモリ使用量
- `cpu_usage_percent`: CPU使用率
- `scrape_samples_scraped`: スクレイプサンプル数
- `scrape_samples_post_metric_relabeling`: ラベリング後サンプル数
- `scrape_duration_seconds`: スクレイプ期間
- `scrape_series_added`: 追加された系列数
- `up`: サービス稼働状態
- `active_connections`: アクティブ接続数

## 実行結果の解釈

### 現在の状況
- rust-appジョブは存在している
- しかし、除外対象以外のカスタムメトリクスは現在送信されていない
- つまり、rust-appは基本的なPrometheusメトリクスのみを送信している状態

### カスタムメトリクスを追加するには

rust-appアプリケーション側で以下のようなカスタムメトリクスを追加する必要があります：

```rust
// Rust例（prometheus crateを使用）
use prometheus::{Counter, Gauge, register_counter, register_gauge};

// カウンタメトリクス
let requests_total = register_counter!(
    "http_requests_total", 
    "Total HTTP requests"
).unwrap();

// ゲージメトリクス  
let queue_size = register_gauge!(
    "task_queue_size",
    "Current task queue size"
).unwrap();

// メトリクスを更新
requests_total.inc();
queue_size.set(42.0);
```

## 使用方法

```bash
# プログラムを実行
cargo run

# 実行スクリプトを使用
./run.sh

# 他の例も実行可能
cargo run --example simple
cargo run --example detailed
```
