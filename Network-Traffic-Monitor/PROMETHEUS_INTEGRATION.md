# Network Traffic Monitor with Prometheus Integration

このプロジェクトは、ネットワークトラフィックを監視し、Prometheusメトリクスとして出力するRustアプリケーションです。

## 機能

- ネットワークインターフェースからのパケットキャプチャ
- Prometheusメトリクスの生成（パケット数、バイト数、帯域幅、プロトコル別統計）
- 定期的なメトリクスログ出力

## ビルド

```bash
cargo build --release
```

## 実行

### 基本実行（rootユーザーまたはsudo権限が必要）

```bash
# デフォルトインターフェース（ens19）でモニタリング開始
sudo ./target/release/network-traffic-monitor

# 特定のインターフェースを指定
sudo ./target/release/network-traffic-monitor -i eth0

# 詳細ログを有効にする
sudo ./target/release/network-traffic-monitor -v
```

### オプション

- `-i, --interface <INTERFACE>`: 監視対象のネットワークインターフェース（デフォルト: ens19）
- `-v, --verbose`: 詳細ログを有効にする
- `-m, --metrics-port <PORT>`: メトリクスサーバーのポート（デフォルト: 9090）
- `-p, --prometheus-url <URL>`: Prometheusサーバーへのメトリクス送信URL（オプション）
- `-t, --prometheus-interval <SECONDS>`: メトリクス更新間隔（デフォルト: 15秒）

## メトリクス

生成されるPrometheusメトリクス：

- `network_packets_total`: 総パケット数
- `network_bytes_total`: 総バイト数
- `network_bandwidth_bps`: 現在の帯域幅（bps）
- `network_packet_size_bytes`: パケットサイズの分布
- `network_packets_ipv4_total`: IPv4パケット数
- `network_packets_ipv6_total`: IPv6パケット数

## 利用可能なインターフェースの確認

```bash
ip link show
# または
ifconfig
```

## 注意事項

- このアプリケーションはパケットキャプチャを行うため、root権限が必要です
- 高負荷な環境では大量のパケットが処理されるため、ログレベルを適切に設定してください
- 本プロダクション環境で使用する前に、リソース使用量とパフォーマンスをテストしてください

## ライセンス

MIT License
