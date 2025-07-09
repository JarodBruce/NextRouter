# TCP Window Size Monitor の設定例

## 基本的な監視コマンド

# eth0インターフェースで60秒間監視
sudo ./target/release/tcp_window_monitor -i eth0

# 特定のポート（HTTP: 80）のみ監視
sudo ./target/release/tcp_window_monitor -i eth0 -p 80

# HTTPSトラフィック（443番ポート）を監視
sudo ./target/release/tcp_window_monitor -i eth0 -p 443

# 長時間監視（5分間）と詳細ログ
sudo ./target/release/tcp_window_monitor -i eth0 -d 300 -v

# 結果をファイルに保存
sudo ./target/release/tcp_window_monitor -i eth0 -o tcp_analysis.json

## 高度な監視設定

# 統計出力間隔を短縮（5秒間隔）
sudo ./target/release/tcp_window_monitor -i eth0 -s 5

# ネットワーク負荷テスト中の監視
sudo ./target/release/tcp_window_monitor -i eth0 -d 120 -s 10 -v -o load_test_results.json

## 用途別設定例

### Webサーバー監視
sudo ./target/release/tcp_window_monitor -i eth0 -p 80 -d 300 -o web_server_analysis.json

### データベースサーバー監視（MySQL: 3306）
sudo ./target/release/tcp_window_monitor -i eth0 -p 3306 -d 600 -o db_server_analysis.json

### 汎用的なTCP監視
sudo ./target/release/tcp_window_monitor -i eth0 -d 180 -s 15 -v

## トラブルシューティング用設定

# 詳細なパケット解析（デバッグモード）
RUST_LOG=debug sudo ./target/release/tcp_window_monitor -i eth0 -v

# 短時間での高頻度測定
sudo ./target/release/tcp_window_monitor -i eth0 -d 60 -s 5 -v

## セキュリティ設定

# 非rootユーザーでの実行（権限設定後）
sudo setcap cap_net_raw,cap_net_admin=eip ./target/release/tcp_window_monitor
./target/release/tcp_window_monitor -i eth0

## 結果の分析

# JSONファイルの内容確認
jq '.' tcp_analysis.json

# 特定の接続の詳細表示
jq '.["192.168.1.100:80-10.0.0.5:54321"]' tcp_analysis.json
