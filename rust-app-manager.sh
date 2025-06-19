#!/bin/bash

# Rust Network Monitor + Prometheus Exporter 起動スクリプト

set -e

sudo apt install -y build-essential gcc

# 色設定
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# ログ関数
info() {
    echo -e "${BLUE}[INFO] $1${NC}"
}

success() {
    echo -e "${GREEN}[SUCCESS] $1${NC}"
}

warning() {
    echo -e "${YELLOW}[WARNING] $1${NC}"
}

error() {
    echo -e "${RED}[ERROR] $1${NC}"
}

# 設定
PROJECT_DIR="Network-Traffic-Monitor"
DEFAULT_INTERFACE="lo"  # ローカルテスト用
DEFAULT_PORT="8080"     # Prometheusが期待するポート
METRICS_PORT="9090"     # 内部メトリクスサーバー用

# 使用方法
show_usage() {
    echo "Rust Network Monitor + Prometheus Exporter 起動スクリプト"
    echo ""
    echo "使用方法: $0 [オプション]"
    echo ""
    echo "オプション:"
    echo "  start [interface] [port]  - アプリケーションを起動 (デフォルト: lo 8080)"
    echo "  build                     - プロジェクトをビルド"
    echo "  stop                      - アプリケーションを停止"
    echo "  status                    - 稼働状態を確認"
    echo "  test                      - 接続テスト"
    echo "  interfaces                - 利用可能なネットワークインターフェースを表示"
    echo "  help                      - このヘルプを表示"
    echo ""
    echo "例:"
    echo "  $0 start lo 8080         # ローカルループバックでテスト"
    echo "  $0 start eth0 8080       # eth0インターフェースを監視"
    echo "  $0 test                  # Prometheusエンドポイントをテスト"
}

# プロジェクトディレクトリの確認
check_project() {
    if [[ ! -d "$PROJECT_DIR" ]]; then
        error "プロジェクトディレクトリが見つかりません: $PROJECT_DIR"
        exit 1
    fi
    
    if [[ ! -f "$PROJECT_DIR/Cargo.toml" ]]; then
        error "Cargo.tomlファイルが見つかりません"
        exit 1
    fi
    
    success "プロジェクトディレクトリを確認しました: $PROJECT_DIR"
}

# Rustの確認とセットアップ
check_rust() {
    if ! command -v cargo &> /dev/null; then
        error "Rustがインストールされていません"
        info "Rustをインストールしてください: https://rustup.rs/"
        exit 1
    fi
    
    # デフォルトのRustツールチェーンが設定されているかチェック
    if ! rustc --version &> /dev/null; then
        warning "Rustのデフォルトツールチェーンが設定されていません"
        info "デフォルトのRustツールチェーンを設定しています..."
        if rustup default stable; then
            success "Rustの安定版ツールチェーンを設定しました"
        else
            error "Rustツールチェーンの設定に失敗しました"
            exit 1
        fi
    fi
    
    info "Rust version: $(rustc --version)"
}

# プロジェクトビルド
build_project() {
    info "プロジェクトをビルドしています..."
    cd "$PROJECT_DIR"
    
    if cargo build --release; then
        success "ビルドが完了しました"
    else
        error "ビルドに失敗しました"
        exit 1
    fi
    
    cd ..
}

# アプリケーション起動
start_app() {
    local interface="${1:-$DEFAULT_INTERFACE}"
    local port="${2:-$DEFAULT_PORT}"
    
    info "アプリケーションを起動しています..."
    info "インターフェース: $interface"
    info "Prometheusポート: $port"
    
    # 既存のプロセスを確認
    if pgrep -f "network-traffic-monitor" > /dev/null; then
        warning "既にアプリケーションが稼働中です"
        return
    fi
    
    cd "$PROJECT_DIR"
    
    # Prometheusの設定に合わせて起動
    info "Rust Network Monitor を起動中..."
    
    # バックグラウンドで起動
    nohup sudo ./target/release/network-traffic-monitor \
        --interface "$interface" \
        --metrics-port "$port" \
        --verbose \
        > ../rust-app.log 2>&1 &
    
    local pid=$!
    echo $pid > ../rust-app.pid
    
    cd ..
    
    # 起動確認
    sleep 3
    if kill -0 $pid 2>/dev/null; then
        success "アプリケーションが起動しました (PID: $pid)"
        info "ログファイル: rust-app.log"
        info "Prometheusエンドポイント: http://localhost:$port/metrics"
    else
        error "アプリケーションの起動に失敗しました"
        error "ログを確認してください: tail -f rust-app.log"
        exit 1
    fi
}

# アプリケーション停止
stop_app() {
    info "アプリケーションを停止しています..."
    
    if [[ -f "rust-app.pid" ]]; then
        local pid=$(cat rust-app.pid)
        if kill -0 $pid 2>/dev/null; then
            sudo kill $pid
            rm -f rust-app.pid
            success "アプリケーションを停止しました"
        else
            warning "プロセスが見つかりません"
            rm -f rust-app.pid
        fi
    else
        # プロセス名で検索して停止
        if pgrep -f "network-traffic-monitor" > /dev/null; then
            sudo pkill -f "network-traffic-monitor"
            success "アプリケーションを停止しました"
        else
            info "停止するプロセスが見つかりません"
        fi
    fi
}

# ステータス確認
check_status() {
    echo "=== Rust Network Monitor ステータス ==="
    
    if pgrep -f "network-traffic-monitor" > /dev/null; then
        success "アプリケーションは稼働中です"
        
        local pid=$(pgrep -f "network-traffic-monitor")
        echo "PID: $pid"
        
        # ポート確認
        if netstat -tulpn 2>/dev/null | grep ":$DEFAULT_PORT " > /dev/null; then
            success "ポート $DEFAULT_PORT で待機中"
        else
            warning "ポート $DEFAULT_PORT が開いていません"
        fi
        
        # 最近のログ
        if [[ -f "rust-app.log" ]]; then
            echo ""
            echo "=== 最近のログ (最新10行) ==="
            tail -n 10 rust-app.log
        fi
    else
        warning "アプリケーションは停止中です"
    fi
}

# 接続テスト
test_connection() {
    echo "=== Prometheus エンドポイント接続テスト ==="
    
    local url="http://localhost:$DEFAULT_PORT/metrics"
    info "テスト URL: $url"
    
    if command -v curl &> /dev/null; then
        if curl -s --max-time 5 "$url" > /dev/null; then
            success "接続成功!"
            echo ""
            echo "=== メトリクス例 (最初の10行) ==="
            curl -s "$url" | head -10
        else
            error "接続失敗"
            info "アプリケーションが起動しているか確認してください"
        fi
    else
        warning "curlがインストールされていません"
        info "手動でブラウザでアクセスしてください: $url"
    fi
}

# ネットワークインターフェース表示
show_interfaces() {
    echo "=== 利用可能なネットワークインターフェース ==="
    ip link show | grep -E '^[0-9]+:' | awk -F': ' '{print "  " $2}' | sed 's/@.*//'
    echo ""
    echo "推奨:"
    echo "  lo    - ローカルテスト用"
    echo "  eth0  - 有線LAN"
    echo "  wlan0 - 無線LAN"
}

# メイン処理
case "${1:-help}" in
    "build")
        check_project
        check_rust
        build_project
        ;;
    "start")
        check_project
        check_rust
        build_project
        start_app "$2" "$3"
        ;;
    "stop")
        stop_app
        ;;
    "status")
        check_status
        ;;
    "test")
        test_connection
        ;;
    "interfaces")
        show_interfaces
        ;;
    "help"|"--help"|"-h")
        show_usage
        ;;
    *)
        error "無効なオプション: $1"
        echo ""
        show_usage
        exit 1
        ;;
esac
