# TCP Window Monitor with Prometheus Metrics

このプロジェクトは、TCPパケットを監視し、パケットロスとウィンドウサイズの縮小を検出してPrometheusメトリクスとして出力するRustアプリケーションです。

## 機能

- **パケットロス検出**: 
  - シーケンス番号の欠損（Missing Sequence）
  - 重複パケット（Duplicate Sequence）
  - 順序違いパケット（Out-of-Order）
  
- **TCPウィンドウサイズ監視**:
  - ウィンドウサイズの縮小イベント検出
  - 現在のウィンドウサイズの追跡

- **Prometheusメトリクス**:
  - カウンターメトリクス（パケット数、パケットロス数など）
  - ゲージメトリクス（アクティブ接続数、ウィンドウサイズなど）
  - ヒストグラムメトリクス（パケットロスのギャップサイズ分布）

## 使用方法

### 基本的な使用方法

```bash
# 管理者権限で実行（パケットキャプチャのため）
sudo ./target/release/tcp_window_monitor -i eth0

# 詳細なログを有効にする
sudo ./target/release/tcp_window_monitor -i eth0 -v

# 統計出力間隔を変更（デフォルト: 1秒）
sudo ./target/release/tcp_window_monitor -i eth0 -s 5

# Prometheusメトリクスポートを変更（デフォルト: 9090）
sudo ./target/release/tcp_window_monitor -i eth0 -p 9100
```

### コマンドライン引数

- `-i, --interface <INTERFACE>`: 監視するネットワークインターフェース名（必須）
- `-s, --stats-interval <SECONDS>`: 統計出力間隔（デフォルト: 1秒）
- `-v, --verbose`: 詳細なログ出力を有効にする
- `-p, --prometheus-port <PORT>`: Prometheusメトリクス用のHTTPポート（デフォルト: 9090）

## Prometheusメトリクス

アプリケーションが起動すると、指定されたポート（デフォルト: 9090）で以下のメトリクスが提供されます：

### エンドポイント
```
http://localhost:9090/metrics
```

### メトリクス一覧

| メトリクス名 | タイプ | 説明 |
|-------------|--------|------|
| `tcp_monitor_total_packets` | Counter | 処理された総パケット数 |
| `tcp_monitor_tcp_packets` | Counter | 処理されたTCPパケット数 |
| `tcp_monitor_global_tcp_packets` | Counter | グローバルIPアドレス間のTCPパケット数 |
| `tcp_monitor_packet_loss_missing` | Counter | シーケンス番号欠損イベント数 |
| `tcp_monitor_packet_loss_duplicate` | Counter | 重複パケットイベント数 |
| `tcp_monitor_packet_loss_out_of_order` | Counter | 順序違いパケットイベント数 |
| `tcp_monitor_window_shrink` | Counter | ウィンドウサイズ縮小イベント数 |
| `tcp_monitor_active_connections` | Gauge | アクティブなTCP接続数 |
| `tcp_monitor_current_window_size` | Gauge | 現在のTCPウィンドウサイズ |
| `tcp_monitor_packet_loss_gap` | Histogram | パケットロスのギャップサイズ分布 |

## Prometheus設定例

Prometheusでメトリクスを収集するには、以下の設定を`prometheus.yml`に追加します：

```yaml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'tcp-monitor'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 5s
```

## Grafanaダッシュボード例

### パケットロス率の可視化

```promql
# パケットロス率（%）
rate(tcp_monitor_packet_loss_missing[1m]) + 
rate(tcp_monitor_packet_loss_duplicate[1m]) + 
rate(tcp_monitor_packet_loss_out_of_order[1m]) / 
rate(tcp_monitor_tcp_packets[1m]) * 100
```

### ウィンドウサイズの監視

```promql
# 現在のウィンドウサイズ
tcp_monitor_current_window_size

# ウィンドウ縮小イベント率
rate(tcp_monitor_window_shrink[1m])
```

### パケットロスタイプ別統計

```promql
# タイプ別パケットロス率
rate(tcp_monitor_packet_loss_missing[1m])
rate(tcp_monitor_packet_loss_duplicate[1m])
rate(tcp_monitor_packet_loss_out_of_order[1m])
```

## ビルド方法

```bash
# 開発版ビルド
cargo build

# リリース版ビルド（最適化あり）
cargo build --release

# 実行
sudo ./target/release/tcp_window_monitor -i eth0
```

## 必要な権限

このアプリケーションは生のパケットキャプチャを行うため、以下の権限が必要です：

1. **root権限**: `sudo`で実行する
2. **CAP_NET_RAW権限**: 特定のユーザーに権限を付与する場合
   ```bash
   sudo setcap cap_net_raw=ep ./target/release/tcp_window_monitor
   ```

## 注意事項

- このツールはグローバルIPアドレス間のTCP通信のみを監視します
- 高トラフィック環境では、パフォーマンスに影響を与える可能性があります
- パケットキャプチャには適切な権限が必要です

## トラブルシューティング

### 権限エラー
```bash
# エラー: Operation not permitted
sudo ./target/release/tcp_window_monitor -i eth0
```

### インターフェースが見つからない
```bash
# 利用可能なインターフェースを確認
ip link show
```

### Prometheusメトリクスにアクセスできない
```bash
# ポートが開いているか確認
netstat -tuln | grep 9090

# メトリクスを確認
curl http://localhost:9090/metrics
```

## 依存関係

主要な依存関係：
- `pcap`: パケットキャプチャ
- `pnet`: ネットワークプロトコル解析
- `prometheus`: メトリクス収集
- `hyper`: HTTPサーバー
- `tokio`: 非同期ランタイム

## ライセンス

このプロジェクトは[MITライセンス](LICENSE)の下で提供されています。
