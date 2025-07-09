# TCP Window Size Monitor

指定したネットワークインターフェースでTCPウィンドウサイズを監視し、回線の限界を把握するためのRustプログラムです。

## 機能

- 指定したネットワークインターフェースでTCPパケットをキャプチャ
- TCPウィンドウサイズの測定と統計分析
- 接続別の詳細統計（最小/最大/平均ウィンドウサイズ、標準偏差）
- リアルタイム統計表示
- 結果のJSONファイル出力
- ポート別フィルタリング機能

## 必要な権限

このプログラムはパケットキャプチャを行うため、root権限または適切なネットワーク権限が必要です。

```bash
# root権限で実行
sudo ./target/release/tcp_window_monitor -i eth0

# または、ユーザーにキャプチャ権限を付与
sudo setcap cap_net_raw,cap_net_admin=eip ./target/release/tcp_window_monitor
```

## ビルド

```bash
# デバッグビルド
cargo build

# リリースビルド（推奨）
cargo build --release
```

## 使用方法

### 基本的な使用例

```bash
# eth0インターフェースで60秒間監視
sudo ./target/release/tcp_window_monitor -i eth0

# 特定のポート（例：80）のみ監視
sudo ./target/release/tcp_window_monitor -i eth0 -p 80

# 結果をファイルに保存
sudo ./target/release/tcp_window_monitor -i eth0 -o tcp_stats.json

# 詳細ログ付きで監視
sudo ./target/release/tcp_window_monitor -i eth0 -v
```

### パラメータ

- `-i, --interface <INTERFACE>`: 監視するネットワークインターフェース名（必須）
- `-d, --duration <DURATION>`: 監視時間（秒）[デフォルト: 60]
- `-s, --stats-interval <STATS_INTERVAL>`: 統計出力間隔（秒）[デフォルト: 10]
- `-o, --output <OUTPUT>`: 結果を保存するJSONファイル名
- `-p, --port <PORT>`: 監視する特定のポート番号
- `-v, --verbose`: 詳細ログを出力

### インターフェース名の確認

利用可能なネットワークインターフェースを確認：

```bash
# Linuxの場合
ip link show

# または
ifconfig -a

# プログラムで確認（詳細モード）
sudo ./target/release/tcp_window_monitor -i dummy -v
```

## 出力例

```
=== TCP Window Size 統計 ===
監視時間: 60.123s
総パケット数: 15432
TCP パケット数: 8765
アクティブ接続数: 12
TCP パケット比率: 56.78%

--- 接続: 192.168.1.100:80-10.0.0.5:54321 ---
  測定回数: 245
  最小ウィンドウサイズ: 8192 bytes
  最大ウィンドウサイズ: 65536 bytes
  平均ウィンドウサイズ: 32768 bytes
  標準偏差: 12345 bytes
  推定最大帯域幅 (Window based): 64.00 KB/s
  最後の観測: 2025-07-09 12:34:56 UTC
```

## 分析のポイント

### ウィンドウサイズから読み取れる情報

1. **小さなウィンドウサイズ**
   - 受信側のバッファ不足
   - アプリケーションの処理遅延
   - ネットワークの輻輳制御

2. **ウィンドウサイズの変動**
   - 動的な帯域幅調整
   - アプリケーションの処理負荷変動
   - ネットワーク状況の変化

3. **推定帯域幅**
   - Window Size ÷ RTT = 理論最大スループット
   - 実際の測定にはRTTの別途測定が必要

### トラブルシューティング

1. **Permission denied エラー**
   ```bash
   sudo setcap cap_net_raw,cap_net_admin=eip ./target/release/tcp_window_monitor
   ```

2. **インターフェースが見つからない**
   - `ip link show` でインターフェース名を確認
   - `-v` オプションで利用可能なインターフェースを表示

3. **パケットがキャプチャされない**
   - インターフェースにトラフィックがあることを確認
   - ファイアウォール設定を確認
   - プロミスキャスモードが有効か確認

## 依存関係

- `pcap`: パケットキャプチャライブラリ
- `pnet`: ネットワークパケット解析
- `clap`: コマンドライン引数解析
- `tokio`: 非同期ランタイム
- `serde`: シリアライゼーション
- `chrono`: 日時処理

## ライセンス

MIT License

## 注意事項

- ネットワーク監視には適切な権限が必要です
- 大量のトラフィックがある環境では、CPU使用率が高くなる場合があります
- プライバシーとセキュリティポリシーに従って使用してください
