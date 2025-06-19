# 🌐 NextRouter - Network Monitoring & Router Setup Suite

Linux環境向けのネットワーク監視・ルーター設定統合ツールセット。Prometheusメトリクス収集、nftablesルーター設定、ネットワークトラフィック監視を一元管理します。

## 📋 構成要素

- **🔥 nftables Router** - IPv4専用シンプルルーター設定（実験用）
- **📊 Prometheus Server** - メトリクス収集・保存システム
- **🦀 Network Traffic Monitor** - Rust製リアルタイムネットワークトラフィック監視
- **🛠️ Management Scripts** - 各種管理・運用スクリプト

## 🚀 クイックスタート

### 1. 前提条件

```bash
# 必要なパッケージがインストールされていることを確認
sudo apt update
sudo apt install -y nftables dnsmasq build-essential gcc curl

# Rustの確認（未インストールの場合は https://rustup.rs/ を参照）
rustc --version
```

### 2. Prometheusセットアップ

```bash
# Prometheus インストール・設定
./prometheus.sh

# 管理ユーティリティ使用
chmod +x prometheus-manager.sh
./prometheus-manager.sh status    # ステータス確認
./prometheus-manager.sh start     # 起動
```

### 3. Grafanaセットアップ（可視化）

```bash
# Grafana インストール・設定
./grafana-setup.sh

# PrometheusデータソースとダッシュボードをGrafanaに設定
./grafana-configure.sh

# 管理ユーティリティ使用
chmod +x grafana-manager.sh
./grafana-manager.sh status     # ステータス確認
./grafana-manager.sh start      # 起動
./grafana-manager.sh dashboard  # ダッシュボードを開く
```

### 4. ネットワーク監視開始

```bash
# Rust監視ツール管理
chmod +x rust-app-manager.sh

# 利用可能なネットワークインターフェース確認
./rust-app-manager.sh interfaces

# 監視開始（例：loopbackインターフェース）
./rust-app-manager.sh start lo 8080
```

### 5. ルーター設定（実験用）

```bash
# IPv4ルーター設定（要root権限）
sudo ./nftables-setup.sh <WAN_INTERFACE> <LAN_INTERFACE>

# 例：eth0をWAN、eth1をLANとする場合
sudo ./nftables-setup.sh eth0 eth1
```

## 🚀 クイック統合セットアップ

全ての機能を一度にセットアップしたい場合：

```bash
# 全コンポーネント統合セットアップ（テスト用）
chmod +x setup-complete.sh
./setup-complete.sh
```

これによりPrometheus + Grafana + ネットワーク監視が自動的にセットアップされ、指定されたメトリクスフィルターを使用したダッシュボードが作成されます。

## 📱 アクセス先

| サービス | URL | 説明 |
|---------|-----|------|
| Prometheus | http://localhost:9090 | メトリクス管理・クエリ |
| Grafana | http://localhost:3000 | ダッシュボード・可視化 |
| Network Monitor | http://localhost:8080/metrics | ネットワークメトリクス |

## 🔧 管理コマンド

### Grafana管理

```bash
# 基本操作
./grafana-manager.sh status     # ステータス確認
./grafana-manager.sh start      # 起動
./grafana-manager.sh stop       # 停止
./grafana-manager.sh restart    # 再起動
./grafana-manager.sh logs       # ログ表示
./grafana-manager.sh dashboard  # ダッシュボードを開く
./grafana-manager.sh configure  # 設定の再実行
```

### Prometheus管理

```bash
# 基本操作
./prometheus-manager.sh status    # ステータス確認
./prometheus-manager.sh start     # 起動
./prometheus-manager.sh stop      # 停止
./prometheus-manager.sh restart   # 再起動
./prometheus-manager.sh reload    # 設定リロード
./prometheus-manager.sh check     # 設定検証
./prometheus-manager.sh logs      # ログ表示
./prometheus-manager.sh test      # 接続テスト
```

### ネットワーク監視管理

```bash
# Rust監視ツール
./rust-app-manager.sh build      # ビルド
./rust-app-manager.sh start [if] [port]  # 起動
./rust-app-manager.sh stop       # 停止
./rust-app-manager.sh status     # ステータス
./rust-app-manager.sh test       # メトリクス接続テスト
./rust-app-manager.sh interfaces # 利用可能IF表示
```

## 📊 監視対象メトリクス

### Prometheusメトリクス
- `prometheus_notifications_total` - 通知総数
- `prometheus_config_last_reload_successful` - 設定リロード状態
- `prometheus_tsdb_compactions_total` - データベース圧縮回数

### ネットワークトラフィックメトリクス
- `network_packets_total` - パケット総数（受信・送信別）
- `network_bytes_total` - バイト総数（受信・送信別）
- `network_errors_total` - ネットワークエラー総数
- `network_interface_up` - インターフェース稼働状態

## 🔨 設定ファイル

### Prometheus設定（prometheus.yml）
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

### ルーター設定
- **LAN IPv4**: 10.40.0.0/24
- **Gateway**: 10.40.0.1
- **DHCP Range**: 10.40.0.100 - 10.40.0.200
- **DNS**: 1.1.1.1 (Cloudflare)

## 🛠️ トラブルシューティング

### Prometheusが起動しない場合

```bash
# ログ確認
./prometheus-manager.sh logs

# 設定ファイル検証
./prometheus-manager.sh check

# サービス状態確認
sudo systemctl status prometheus
```

### Grafanaが起動しない場合

```bash
# ログ確認
./grafana-manager.sh logs

# 設定ファイル検証
./grafana-manager.sh check

# サービス状態確認
sudo systemctl status grafana-server
```

### ネットワーク監視が動作しない場合

```bash
# インターフェース確認
./rust-app-manager.sh interfaces

# 権限確認（一部のインターフェースはsudo権限が必要）
sudo ./rust-app-manager.sh start eth0 8080

# 接続テスト
./rust-app-manager.sh test
```

### ルーター設定の問題

```bash
# nftables設定確認
sudo nft list ruleset

# IP転送確認
cat /proc/sys/net/ipv4/ip_forward

# インターフェース状態確認
ip addr show
ip route show
```

## 🏗️ プロジェクト構造

```
NextRouter/
├── Network-Traffic-Monitor/        # Rust製ネットワーク監視ツール
│   ├── src/
│   │   ├── main.rs                # メインアプリケーション
│   │   ├── capture.rs             # パケットキャプチャ
│   │   ├── stats.rs               # 統計処理
│   │   └── prometheus_server.rs   # Prometheusサーバー
│   ├── Cargo.toml                 # Rust依存関係
│   └── target/                    # ビルド成果物
├── prometheus.yml                  # Prometheus設定ファイル
├── prometheus.sh                   # Prometheusインストールスクリプト
├── prometheus-manager.sh           # Prometheus管理ユーティリティ
├── grafana-setup.sh               # Grafanaインストールスクリプト
├── grafana-manager.sh              # Grafana管理ユーティリティ
├── rust-app-manager.sh            # Rust監視ツール管理スクリプト
├── nftables-setup.sh              # ルーター設定スクリプト
├── prometheus-install.md          # Prometheus詳細インストールガイド
└── README.md                      # このファイル
```

## ⚠️ 注意事項

- **ルーター機能は実験用です**。本番環境での使用は推奨されません。
- **root権限が必要な操作**：nftables設定、一部のネットワークインターフェース監視
- **IPv6は無効化**されます（ルーター設定時）
- **ファイアウォール設定**が変更されるため、既存のネットワーク設定に影響する可能性があります

## 📚 詳細ドキュメント

- [Prometheus詳細インストールガイド](prometheus-install.md)
- [Network Traffic Monitor技術仕様](Network-Traffic-Monitor/TRAFFIC_MONITOR.md)
- [Prometheus統合ガイド](Network-Traffic-Monitor/PROMETHEUS_INTEGRATION.md)

## 📝 ライセンス

このプロジェクトはMITライセンスの下で公開されています。