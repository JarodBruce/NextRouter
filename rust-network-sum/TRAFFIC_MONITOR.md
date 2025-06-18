# Network Traffic Monitor

ens19インターフェース（およびその他のネットワークインターフェース）に流れるトラフィックをリアルタイムで監視・集計するRustプログラムです。

## 機能

- **リアルタイムパケット監視**: 指定されたネットワークインターフェースのパケットをキャプチャ
- **プロトコル別統計**: TCP、UDP、ICMP、その他のプロトコル別にトラフィックを集計
- **IPアドレス統計**: 送信元・宛先IPアドレス別のトラフィック量を記録
- **ポート統計**: よく使用されるポート番号の統計
- **JSON出力**: 統計データをJSON形式でファイルに保存
- **定期的なレポート**: 指定した間隔で統計レポートを生成

## 前提条件

- Linux系OS（Ubuntu、Debian等）
- Rust 1.70以上
- root権限（パケットキャプチャには管理者権限が必要）
- libpcap開発ライブラリ

## インストール

### 依存パッケージのインストール（Ubuntu/Debian）

```bash
sudo apt update
sudo apt install libpcap-dev build-essential
```

### Rustプロジェクトのビルド

```bash
cargo build --release
```

## 使用方法

### 基本的な使用法

```bash
# デフォルト設定（ens19インターフェース、60秒間隔）
sudo ./target/release/network-traffic-monitor

# カスタムインターフェースを指定
sudo ./target/release/network-traffic-monitor -i eth0

# 統計出力間隔を変更（30秒間隔）
sudo ./target/release/network-traffic-monitor -i ens19 -s 30

# 出力ファイルを指定
sudo ./target/release/network-traffic-monitor -i ens19 -o /var/log/traffic_stats.json

# 詳細ログを有効化
sudo ./target/release/network-traffic-monitor -i ens19 -v
```

### コマンドラインオプション

```
Options:
  -i, --interface <INTERFACE>  Network interface to monitor (default: ens19)
  -o, --output <OUTPUT>        Output file for statistics (default: traffic_stats.json)
  -s, --interval <INTERVAL>    Statistics aggregation interval in seconds (default: 60)
  -v, --verbose               Enable verbose logging
  -h, --help                  Print help
  -V, --version               Print version
```

## 出力例

### コンソール出力

```
INFO - Starting network traffic monitor
INFO - Interface: ens19
INFO - Output file: traffic_stats.json
INFO - Statistics interval: 60 seconds
INFO - Traffic Statistics:
INFO -   Total: 15234 packets, 12456789 bytes
INFO -   TCP: 12891 packets, 11234567 bytes
INFO -   UDP: 2134 packets, 1034567 bytes
INFO -   ICMP: 156 packets, 156789 bytes
INFO -   Other: 53 packets, 30866 bytes
INFO - Statistics saved to: traffic_stats.json
```

### JSON出力例

```json
{
  "timestamp": "2025-06-17T10:30:00Z",
  "interface": "ens19",
  "total_packets": 15234,
  "total_bytes": 12456789,
  "tcp_packets": 12891,
  "tcp_bytes": 11234567,
  "udp_packets": 2134,
  "udp_bytes": 1034567,
  "icmp_packets": 156,
  "icmp_bytes": 156789,
  "other_packets": 53,
  "other_bytes": 30866,
  "top_src_ips": [
    ["192.168.1.100", 5678912],
    ["192.168.1.101", 3456789],
    ["10.0.0.50", 1234567]
  ],
  "top_dst_ips": [
    ["8.8.8.8", 2345678],
    ["1.1.1.1", 1234567],
    ["192.168.1.1", 987654]
  ],
  "top_ports": [
    [80, 8756],
    [443, 6543],
    [53, 2341]
  ]
}
```

## nftablesとの連携

このプログラムは、nftablesでNAPTを設定しているルーター環境で特に有用です。`nftables-setup.sh`で設定されたルーターの通信を監視できます。

### 監視対象の設定例

```bash
# WANインターフェースの監視
sudo ./target/release/network-traffic-monitor -i enxc8a362d31ba2

# LANインターフェースの監視
sudo ./target/release/network-traffic-monitor -i enp1s0

# 両方のインターフェースを同時監視（別ターミナルで実行）
sudo ./target/release/network-traffic-monitor -i enxc8a362d31ba2 -o wan_stats.json &
sudo ./target/release/network-traffic-monitor -i enp1s0 -o lan_stats.json &
```

## systemdサービスとしての運用

継続的な監視のために、systemdサービスとして設定することも可能です：

```bash
# サービスファイルの作成
sudo tee /etc/systemd/system/traffic-monitor.service > /dev/null <<EOF
[Unit]
Description=Network Traffic Monitor
After=network.target

[Service]
Type=simple
User=root
ExecStart=/path/to/network-traffic-monitor -i ens19 -o /var/log/traffic_stats.json
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

# サービスの有効化・開始
sudo systemctl daemon-reload
sudo systemctl enable traffic-monitor
sudo systemctl start traffic-monitor
```

## ログ分析

JSONログは、jqコマンドを使って簡単に分析できます：

```bash
# 最新の統計を表示
tail -n 1 traffic_stats.json | jq .

# TCP通信量の推移を表示
jq '.tcp_bytes' traffic_stats.json

# 上位送信元IPを表示
jq '.top_src_ips' traffic_stats.json
```

## 注意事項

- このプログラムは教育・実験目的で作成されています
- 本番環境での使用前に十分なテストを行ってください
- root権限が必要なため、セキュリティに注意してください
- 大量のトラフィックがある環境では、ファイルサイズが急速に増大する可能性があります

## トラブルシューティング

### パーミッションエラー

```bash
# raw socketの使用権限を確認
sudo setcap cap_net_raw,cap_net_admin=eip ./target/release/network-traffic-monitor

# または常にsudoで実行
sudo ./target/release/network-traffic-monitor
```

### インターフェースが見つからない

```bash
# 利用可能なインターフェースを確認
ip link show
```

### 依存関係エラー

```bash
# libpcap開発ライブラリの再インストール
sudo apt install --reinstall libpcap-dev
```
