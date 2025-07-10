#!/bin/bash

# TCP Window Monitor with Prometheus Metrics - 実行例

echo "TCP Window Monitor with Prometheus Metrics"
echo "=========================================="
echo ""

# インターフェース一覧を表示
echo "利用可能なネットワークインターフェース:"
ip link show | grep -E '^[0-9]+:' | awk '{print $2}' | sed 's/://'
echo ""

# 必要なパッケージのインストール
sudo apt update
sudo apt install libpcap-dev -y

# デフォルトのインターフェースを取得
DEFAULT_INTERFACE=$(ip route | grep default | awk '{print $5}' | head -n1)
echo "デフォルトインターフェース: $DEFAULT_INTERFACE"
echo ""

# 実行例を表示
echo "実行例:"
echo "--------"
echo ""
echo "1. 基本的な使用方法："
echo "   sudo ./target/release/tcp_window_monitor -i $DEFAULT_INTERFACE"
echo ""
echo "2. 詳細ログ付きで実行："
echo "   sudo ./target/release/tcp_window_monitor -i $DEFAULT_INTERFACE -v"
echo ""
echo "3. 統計出力間隔を5秒に設定："
echo "   sudo ./target/release/tcp_window_monitor -i $DEFAULT_INTERFACE -s 5"
echo ""
echo "4. Prometheusポートを9100に変更："
echo "   sudo ./target/release/tcp_window_monitor -i $DEFAULT_INTERFACE -p 9100"
echo ""
echo "5. 全オプションを指定："
echo "   sudo ./target/release/tcp_window_monitor -i $DEFAULT_INTERFACE -s 5 -p 9100 -v"
echo ""

# Prometheusメトリクスの確認方法
echo "Prometheusメトリクスの確認:"
echo "-------------------------"
echo ""
echo "アプリケーション起動後、以下のコマンドでメトリクスを確認できます："
echo "curl http://localhost:9090/metrics"
echo ""
echo "または、ブラウザで以下のURLを開いてください："
echo "http://localhost:9090/metrics"
echo ""

# 監視対象の説明
echo "監視対象:"
echo "--------"
echo "このツールは以下を監視します："
echo "- グローバルIPアドレス間のTCP通信のみ"
echo "- パケットロス（シーケンス番号欠損、重複、順序違い）"
echo "- TCPウィンドウサイズの縮小イベント"
echo "- アクティブなTCP接続数"
echo ""

# 権限について
echo "権限について:"
echo "-------------"
echo "このアプリケーションは生のパケットキャプチャを行うため、"
echo "管理者権限（sudo）が必要です。"
echo ""
echo "特定のユーザーに権限を付与する場合："
echo "sudo setcap cap_net_raw=ep ./target/release/tcp_window_monitor"
echo ""

# 実際に実行するかユーザーに確認
echo "実際に実行しますか？ (y/N)"
read -r response
if [[ "$response" =~ ^[Yy]$ ]]; then
    echo ""
    echo "アプリケーションを起動しています..."
    echo "Ctrl+C で停止できます"
    echo ""
    
    # リリースビルドが存在するか確認
    if [ ! -f "./target/release/tcp_window_monitor" ]; then
        echo "リリースビルドが見つかりません。ビルドを実行します..."
        cargo build --release
    fi
    
    # アプリケーションを実行
    sudo ./target/release/tcp_window_monitor -i "$DEFAULT_INTERFACE" -v
else
    echo "実行をキャンセルしました。"
fi
