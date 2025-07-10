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
DEFAULT_INTERFACE="eth2"  # ローカルテスト用

# 使用方法
show_usage() {
    echo "Rust Network Monitor + Prometheus Exporter 起動スクリプト"
    echo ""
    echo "使用方法: $0 [オプション]"
    echo ""
    echo "オプション:"
    echo "  start [interface]         - アプリケーションを起動"
    echo "  build                     - プロジェクトをビルド"
    echo "  stop                      - アプリケーションを停止"
    echo "  status                    - 稼働状態を確認"
    echo "  test                      - 接続テスト"
    echo "  interfaces                - 利用可能なネットワークインターフェースを表示"
    echo "  help                      - このヘルプを表示"
    echo ""
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
    if ! command -v cargo >/dev/null 2>&1; then
        error "Rustがインストールされていません"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        
        # 環境変数を設定
        source "$HOME/.cargo/env"
        
        success "Rustをインストールしました"
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
    
    info "アプリケーションを起動しています..."
    info "インターフェース: $interface"
    
    cd "$PROJECT_DIR"
    
    # Prometheusの設定に合わせて起動
    info "Rust Network Monitor を起動中..."
    
    # フォアグラウンドで起動
    sudo ./target/release/network-traffic-monitor \
        --interface "$interface"
}

# アプリケーション停止
stop_app() {
    sudo pkill -9 -f rust-app-manager
    info "アプリケーションを停止しています..."
}

# ステータス確認
check_status() {
    echo "=== Rust Network Monitor ステータス ==="
    
    if pgrep -f "network-traffic-monitor" > /dev/null; then
        success "アプリケーションは稼働中です"
        
        local pid=$(pgrep -f "network-traffic-monitor")
        echo "PID: $pid"
    else
        warning "アプリケーションは停止中です"
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
