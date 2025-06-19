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

# 詳細ログを有効化
sudo ./target/release/network-traffic-monitor -i ens19 -v
```

### コマンドラインオプション

```
Options:
  -i, --interface <INTERFACE>  Network interface to monitor (default: ens19)
  -v, --verbose               Enable verbose logging
  -h, --help                  Print help
  -V, --version               Print version
```