# Network Traffic Monitor

ens19インターフェース（およびその他のネットワークインターフェース）に流れるトラフィックをリアルタイムで監視・集計するRustプログラムです。

## 機能

- **リアルタイムパケット監視**: 指定されたネットワークインターフェースのパケットをキャプチャ
- **プロトコル別統計**: IPv4、IPv6、その他のプロトコル別にトラフィックを集計
- **IPアドレス統計**: 送信元・宛先IPアドレス別のトラフィック量を記録（ローカルIPフィルタリング対応）
- **ポート統計**: よく使用されるポート番号の統計
- **リアルタイム表示**: 1秒間隔でのトラフィック統計をコンソールに表示
- **転送レート表示**: パケット/秒、バイト/秒の転送レートを自動計算
- **非同期処理**: Tokioを使用した効率的な非同期パケット処理

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
# デフォルト設定（ens19インターフェース、1秒間隔で統計表示）
sudo ./target/release/network-traffic-monitor

# カスタムインターフェースを指定
sudo ./target/release/network-traffic-monitor -i eth0

# 統計出力間隔を変更（30秒間隔） ※現在は常に1秒間隔で表示
sudo ./target/release/network-traffic-monitor -i ens19 -s 30

# 詳細ログを有効化
sudo ./target/release/network-traffic-monitor -i ens19 -v
```

### コマンドラインオプション

```
Options:
  -i, --interface <INTERFACE>  Network interface to monitor (default: ens19)
  -s, --interval <INTERVAL>    Statistics aggregation interval in seconds (default: 5)
  -v, --verbose               Enable verbose logging
  -h, --help                  Print help
  -V, --version               Print version
```

## 出力例

## 出力例

### コンソール出力

```
INFO - Starting network traffic monitor
INFO - Interface: ens19
INFO - Statistics interval: 5 seconds
INFO - === Traffic Statistics ===
INFO - Last Second | Rate: 15.2 packets/s, 12.45 KB/s
INFO -   IPv4: 148 packets, 123.45 KB (14.2 KB/s)
INFO -   IPv6: 23 packets, 8.91 KB (1.8 KB/s)
INFO -   Top Local Source IPs:
INFO -     192.168.1.100 - 45.67 KB
INFO -     10.0.0.15 - 23.45 KB
INFO -   Top Local Destination IPs:
INFO -     192.168.1.1 - 67.89 KB
INFO -     10.0.0.1 - 34.56 KB
INFO - ========================
```

## 機能詳細

### トラフィック統計

- **パケット数とバイト数**: 各プロトコルの詳細な統計
- **転送レート**: パケット/秒、バイト/秒の自動計算
- **差分計算**: 前回からの差分を基にした1秒間隔の統計

### IPアドレスフィルタリング

プログラムは以下のローカル/プライベートIPアドレスを自動的に識別してフィルタリングします：

- プライベートIPアドレス（10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16）
- ループバックアドレス（127.0.0.0/8）
- リンクローカルアドレス（169.254.0.0/16）
- IPv6のローカルアドレス
- 監視対象インターフェース自身のIPアドレス

### プロトコル識別

- IPv4/IPv6パケットの自動識別
- TCP/UDP/ICMPプロトコルの詳細解析
- ポート番号の統計（送信元・宛先両方）

## トラブルシューティング

### 権限エラー

```bash
# プログラムを実行する際は必ずroot権限が必要
sudo ./target/release/network-traffic-monitor
```

### インターフェースが見つからない場合

```bash
# 利用可能なインターフェースを確認
ip link show

# プログラムは利用可能なインターフェースをログに出力します
sudo ./target/release/network-traffic-monitor -i invalid_interface -v
```

### 依存関係エラー

```bash
# libpcap開発ライブラリの再インストール
sudo apt install --reinstall libpcap-dev

# ビルドエラーが発生した場合
cargo clean
cargo build --release
```

### パフォーマンス調整

- 高トラフィック環境では、統計間隔を長くすることを推奨
- メモリ使用量を抑えるため、IP統計は上位のみ表示
- バックグラウンド処理により、パケットドロップを最小化

## 技術仕様

### 依存関係

- **tokio**: 非同期ランタイム
- **pnet**: ネットワークパケット処理
- **clap**: コマンドライン引数解析
- **serde/serde_json**: JSON形式でのデータ構造化
- **chrono**: 時刻処理
- **anyhow**: エラーハンドリング
- **log/env_logger**: ログ出力

### アーキテクチャ

- **パケットキャプチャ**: 専用スレッドでバックグラウンド処理
- **統計処理**: 非同期タスクによる効率的な集計
- **表示更新**: 1秒間隔での定期的な統計更新
- **メモリ管理**: Arc/Mutexによる安全な共有状態管理
