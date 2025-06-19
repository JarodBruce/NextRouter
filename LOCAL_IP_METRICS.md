# ローカルIP別トラフィック監視 - Prometheusメトリクス

## 概要

このRustアプリケーションは、ネットワークトラフィックをローカルIP単位で監視し、Prometheusメトリクスとして出力します。

## 追加されたメトリクス

### ローカルIP別送信メトリクス

```promql
# ローカルIPから外部への送信バイト数
local_ip_tx_bytes_total{local_ip="192.168.1.100", remote_ip="8.8.8.8"}

# ローカルIPから外部への送信パケット数  
local_ip_tx_packets_total{local_ip="192.168.1.100", remote_ip="8.8.8.8"}
```

### ローカルIP別受信メトリクス

```promql
# 外部からローカルIPへの受信バイト数
local_ip_rx_bytes_total{local_ip="192.168.1.100", remote_ip="8.8.8.8"}

# 外部からローカルIPへの受信パケット数
local_ip_rx_packets_total{local_ip="192.168.1.100", remote_ip="8.8.8.8"}
```

### ローカルネットワーク範囲の自動判定

アプリケーションは以下のプライベートIPアドレス範囲を自動的にローカルとして認識します：

- `10.0.0.0/8` (10.0.0.0 - 10.255.255.255)
- `172.16.0.0/12` (172.16.0.0 - 172.31.255.255)
- `192.168.0.0/16` (192.168.0.0 - 192.168.255.255)
- `127.0.0.0/8` (localhost/loopback)

## Prometheusクエリ例

### 基本的なクエリ

```promql
# 全ローカルIPの送信バイト数合計
sum by (local_ip) (local_ip_tx_bytes_total)

# 全ローカルIPの受信バイト数合計
sum by (local_ip) (local_ip_rx_bytes_total)

# 特定のローカルIPの通信量
local_ip_tx_bytes_total{local_ip="192.168.1.100"}
```

### 通信率の計算

```promql
# ローカルIPごとの送信率（bytes/sec）
rate(local_ip_tx_bytes_total[5m]) * 8  # bps に変換

# ローカルIPごとの受信率（bytes/sec）
rate(local_ip_rx_bytes_total[5m]) * 8  # bps に変換

# パケット送信率（packets/sec）
rate(local_ip_tx_packets_total[5m])
```

### Top N 分析

```promql
# 送信量が最も多いローカルIP（Top 5）
topk(5, sum by (local_ip) (local_ip_tx_bytes_total))

# 通信先が最も多いローカルIP（Top 10）
topk(10, count by (local_ip) (local_ip_tx_bytes_total))

# 最も通信量が多い通信ペア（Top 10）
topk(10, local_ip_tx_bytes_total)
```

### 時間範囲分析

```promql
# 過去1時間のローカルIP別送信量
increase(local_ip_tx_bytes_total[1h])

# 過去24時間のローカルIP別平均送信率
rate(local_ip_tx_bytes_total[24h]) * 8
```

### アラート用クエリ

```promql
# 異常に多い通信量を検出（閾値: 100MB/5min）
rate(local_ip_tx_bytes_total[5m]) * 300 > 100 * 1024 * 1024

# 新しい通信先への接続を検出
increase(local_ip_tx_packets_total[5m]) > 0 and
  (local_ip_tx_packets_total offset 10m) == 0
```

## Grafanaダッシュボード設定例

### パネル1: ローカルIP別送信量（時系列）

```promql
sum by (local_ip) (rate(local_ip_tx_bytes_total[5m]) * 8)
```

### パネル2: Top 10 ローカルIP（表形式）

```promql
topk(10, sum by (local_ip) (local_ip_tx_bytes_total))
```

### パネル3: ローカルIP別通信先数（ゲージ）

```promql
count by (local_ip) (local_ip_tx_bytes_total)
```

## 実用的な使用例

### 1. 帯域幅監視

各ローカルIPの使用帯域幅を監視し、帯域制限やQoS設定の参考にする：

```promql
# 5分間平均の送信帯域幅（Mbps）
sum by (local_ip) (rate(local_ip_tx_bytes_total[5m]) * 8 / 1024 / 1024)
```

### 2. セキュリティ監視

異常な通信パターンの検出：

```promql
# 通常時の10倍以上の通信量を検出
rate(local_ip_tx_bytes_total[5m]) > 
  10 * rate(local_ip_tx_bytes_total[1h] offset 1d)
```

### 3. リソース最適化

最も通信量の多いデバイスを特定し、ネットワーク最適化の対象とする：

```promql
# 総通信量でランキング
topk(5, 
  sum by (local_ip) (
    local_ip_tx_bytes_total + local_ip_rx_bytes_total
  )
)
```

## 設定のカスタマイズ

### カスタムローカルネットワーク範囲

独自のネットワーク範囲がある場合は、capture.rsの`local_network_ranges`を変更してください：

```rust
let local_network_ranges = vec![
    ("10.40.0.0".parse().unwrap(), 24),    // カスタム範囲: 10.40.0.0/24
    ("192.168.100.0".parse().unwrap(), 24), // カスタム範囲: 192.168.100.0/24
    // ...
];
```

## 注意事項

1. **パフォーマンス**: 大量のトラフィックでは多数のメトリクスラベルが生成される可能性があります
2. **プライバシー**: IPv6アドレスはプレフィックスのみ記録してプライバシーを保護
3. **ストレージ**: 長期間の保存により、Prometheusのストレージ使用量が増加する可能性があります

## トラブルシューティング

### メトリクスが表示されない場合

1. 適切なネットワークインターフェースが選択されているか確認
2. root権限でアプリケーションが実行されているか確認
3. ローカルIPアドレス範囲の設定が正しいか確認

### 大量のメトリクスが生成される場合

カーディナリティを制限するため、以下を検討：

1. 特定のローカルIP範囲のみ監視
2. 外部IPアドレスのマスキング（例：/24単位）
3. しきい値以下の通信量を無視する設定

---

**作成日**: $(date)  
**バージョン**: 0.1.0
