# Prometheus インストールガイド

## 概要

このスクリプトは、現在のディレクトリにある `prometheus.yml` 設定ファイルを使用してPrometheusをインストール・設定します。

## 前提条件

- Linux システム (Ubuntu/Debian推奨)
- sudo権限を持つユーザー
- インターネット接続
- `prometheus.yml` ファイルが現在のディレクトリに存在すること

## インストール手順

### 1. スクリプトの実行権限を付与

```bash
chmod +x prometheus.sh
```

### 2. スクリプトの実行

```bash
./prometheus.sh
```

### 3. インストール確認

```bash
# サービス状態確認
sudo systemctl status prometheus

# Webインターフェースアクセス
# ブラウザで http://localhost:9090 にアクセス
```

## 設定ファイルについて

### 現在の prometheus.yml

スクリプトは現在のディレクトリにある `prometheus.yml` を使用します：

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'prometheus'
    static_configs:
      - targets: ['localhost:9090']

  - job_name: 'rust-app'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
    scrape_interval: 5s
```

この設定では以下のターゲットを監視します：
- **prometheus**: Prometheus自体 (localhost:9090)
- **rust-app**: Rustアプリケーション (localhost:8080)

### 設定のカスタマイズ

必要に応じて `prometheus.yml` を編集してから、スクリプトを実行してください。

## インストール後の管理

### サービス管理

```bash
# Prometheus起動
sudo systemctl start prometheus

# Prometheus停止
sudo systemctl stop prometheus

# Prometheus再起動
sudo systemctl restart prometheus

# 起動状態確認
sudo systemctl status prometheus

# ログ確認
sudo journalctl -u prometheus -f
```

### 設定変更

```bash
# 設定ファイル編集
sudo nano /etc/prometheus/prometheus.yml

# 設定検証
promtool check config /etc/prometheus/prometheus.yml

# 設定リロード (再起動不要)
curl -X POST http://localhost:9090/-/reload
```

## トラブルシューティング

### 1. サービスが起動しない

```bash
# エラーログ確認
sudo journalctl -u prometheus -n 50

# 設定ファイル検証
promtool check config /etc/prometheus/prometheus.yml
```

### 2. ターゲットに接続できない

1. Prometheus Web UI (http://localhost:9090) の Status → Targets でターゲット状態を確認
2. ターゲットアプリケーションが稼働しているか確認
3. ファイアウォール設定を確認

### 3. アンインストール

```bash
# サービス停止・無効化
sudo systemctl stop prometheus
sudo systemctl disable prometheus

# ファイル削除
sudo rm -rf /opt/prometheus
sudo rm -rf /var/lib/prometheus
sudo rm -rf /etc/prometheus
sudo rm /etc/systemd/system/prometheus.service

# ユーザー削除
sudo userdel prometheus

# systemd設定リロード
sudo systemctl daemon-reload
```

## よく使用するPrometheusクエリ

### 基本的なクエリ

```promql
# 全ターゲットの稼働状態
up

# HTTP リクエスト率（5分間平均）
rate(http_requests_total[5m])

# メモリ使用量
process_resident_memory_bytes

# Prometheus設定リロード成功状態
prometheus_config_last_reload_successful
```

### Rustアプリケーション用クエリ

```promql
# Rustアプリケーションの稼働状態
up{job="rust-app"}

# カスタムメトリクス（アプリケーション固有）
rust_app_requests_total
rust_app_processing_duration_seconds
```

## 参考リンク

- [Prometheus公式ドキュメント](https://prometheus.io/docs/)
- [PromQL クエリ言語](https://prometheus.io/docs/prometheus/latest/querying/)
- [設定ファイル仕様](https://prometheus.io/docs/prometheus/latest/configuration/configuration/)
