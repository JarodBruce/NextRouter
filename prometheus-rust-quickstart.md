# 🚀 Rust + Prometheus クイックスタートガイド

## 概要

Prometheus + Rust Network Monitor が正常に起動しました！
Rustアプリケーションがlocalhost:8080でPrometheusメトリクスを提供し、Prometheusがそれらのメトリクスを収集しています。

## 🔗 アクセス先

### Prometheus Web UI
**URL**: http://localhost:9090
- メトリクス確認
- クエリ実行
- ターゲット監視状況確認

### Rust Application Metrics
**URL**: http://localhost:8080/metrics
- 生のメトリクスデータ
- Prometheusが収集するデータソース

### その他のエンドポイント
- **Health Check**: http://localhost:8080/health
- **Root**: http://localhost:8080

## 📊 利用可能なメトリクス

現在のRustアプリケーションが提供するメトリクス：

```promql
# HTTPリクエスト総数
http_requests_total

# アクティブな接続数
active_connections

# CPU使用率 (シミュレート)
cpu_usage_percent

# メモリ使用量 (シミュレート)  
memory_usage_bytes

# リクエスト処理時間
request_duration_seconds
```

## 🎯 Prometheusクエリ例

### 基本的なクエリ
```promql
# 全ターゲットの稼働状態
up

# Rustアプリケーションの稼働状態
up{job="rust-app"}

# HTTPリクエスト率（5分間平均）
rate(http_requests_total[5m])

# CPUメトリクス
cpu_usage_percent

# メモリメトリクス
memory_usage_bytes
```

### 高度なクエリ
```promql
# リクエスト処理時間の平均（5分間）
rate(request_duration_seconds_sum[5m]) / rate(request_duration_seconds_count[5m])

# アクティブ接続数の最大値（1時間）
max_over_time(active_connections[1h])
```

## 🛠️ 管理コマンド

### Prometheus管理
```bash
# ステータス確認
./prometheus-manager.sh status

# サービス操作
./prometheus-manager.sh start|stop|restart

# 設定リロード
./prometheus-manager.sh reload

# 接続テスト
./prometheus-manager.sh test
```

### Rustアプリケーション管理
```bash
# ステータス確認
./rust-app-manager.sh status

# 起動・停止
./rust-app-manager.sh start [interface] [port]
./rust-app-manager.sh stop

# 接続テスト
./rust-app-manager.sh test

# 利用可能インターフェース表示
./rust-app-manager.sh interfaces
```

## 🔧 トラブルシューティング

### 1. Rustアプリケーションが応答しない

```bash
# プロセス確認
./rust-app-manager.sh status

# ログ確認
tail -f rust-app.log

# 再起動
./rust-app-manager.sh stop
./rust-app-manager.sh start
```

### 2. Prometheusがターゲットに接続できない

```bash
# Prometheus設定確認
./prometheus-manager.sh check

# ターゲット手動テスト
curl http://localhost:8080/metrics

# 設定リロード
./prometheus-manager.sh reload
```

### 3. ポート競合の場合

```bash
# 使用中のポート確認
netstat -tulpn | grep :8080
netstat -tulpn | grep :9090

# 別のポートで起動
./rust-app-manager.sh start lo 8081
```

## 📈 モニタリングの拡張

### 1. 新しいメトリクスの追加

`Network-Traffic-Monitor/src/prometheus_server.rs` を編集してカスタムメトリクスを追加できます。

### 2. アラートルールの設定

`/etc/prometheus/rules/` ディレクトリにアラートルールを追加：

```yaml
groups:
  - name: rust-app-alerts
    rules:
      - alert: RustAppDown
        expr: up{job="rust-app"} == 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Rust application is down"
```

### 3. より多くのターゲットの追加

`prometheus.yml` を編集して他のサービスを監視：

```yaml
scrape_configs:
  - job_name: 'node-exporter'
    static_configs:
      - targets: ['localhost:9100']
      
  - job_name: 'my-other-app'
    static_configs:
      - targets: ['localhost:8081']
```

## 🎉 成功！

✅ Prometheus: http://localhost:9090  
✅ Rust App Metrics: http://localhost:8080/metrics  
✅ 自動メトリクス収集  
✅ 管理ツール完備  

これで完全なPrometheus + Rust監視システムが動作しています！

## 📚 参考情報

- [Prometheus公式ドキュメント](https://prometheus.io/docs/)
- [PromQL クエリガイド](https://prometheus.io/docs/prometheus/latest/querying/)
- [Rust Prometheusクレート](https://docs.rs/prometheus/)

---

**作成日時**: $(date)  
**システム**: Prometheus v2.45.0 + Rust Network Monitor v0.1.0
