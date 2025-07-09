#!/bin/bash

# TCP Window Size Monitor セットアップと実行スクリプト

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY_PATH="$SCRIPT_DIR/target/release/tcp_window_monitor"

# 色付きログ関数
log_info() {
    echo -e "\033[32m[INFO]\033[0m $1"
}

log_warn() {
    echo -e "\033[33m[WARN]\033[0m $1"
}

log_error() {
    echo -e "\033[31m[ERROR]\033[0m $1"
}

# 使用方法を表示
show_usage() {
    echo "使用方法: $0 [options]"
    echo ""
    echo "オプション:"
    echo "  build                 プログラムをビルド"
    echo "  setup                 必要な権限を設定"
    echo "  run <interface>       指定したインターフェースで監視を開始"
    echo "  help                  このヘルプを表示"
    echo ""
    echo "例:"
    echo "  $0 build              # プログラムをビルド"
    echo "  $0 setup              # 権限設定"
    echo "  $0 run eth0           # eth0で監視"
}

# プログラムをビルド
build_program() {
    log_info "Rustプログラムをビルドしています..."
    cd "$SCRIPT_DIR"
    
    if ! command -v cargo &> /dev/null; then
        log_error "Rustがインストールされていません"
        log_info "Rustのインストール方法:"
        echo "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi
    
    # 必要なシステムライブラリの確認
    log_info "システム依存関係を確認しています..."
    
    if command -v apt-get &> /dev/null; then
        # Ubuntu/Debian
        if ! dpkg -l | grep -q libpcap-dev; then
            log_warn "libpcap-devがインストールされていません"
            log_info "インストールコマンド: sudo apt-get install libpcap-dev"
        fi
    elif command -v yum &> /dev/null; then
        # CentOS/RHEL
        if ! rpm -qa | grep -q libpcap-devel; then
            log_warn "libpcap-develがインストールされていません"
            log_info "インストールコマンド: sudo yum install libpcap-devel"
        fi
    fi
    
    cargo build --release
    
    if [ -f "$BINARY_PATH" ]; then
        log_info "ビルド完了: $BINARY_PATH"
        ls -la "$BINARY_PATH"
    else
        log_error "ビルドに失敗しました"
        exit 1
    fi
}

# 必要な権限を設定
setup_permissions() {
    if [ ! -f "$BINARY_PATH" ]; then
        log_error "バイナリが見つかりません。まずビルドしてください: $0 build"
        exit 1
    fi
    
    log_info "パケットキャプチャ権限を設定しています..."
    
    if [ "$EUID" -eq 0 ]; then
        # rootで実行中
        setcap cap_net_raw,cap_net_admin=eip "$BINARY_PATH"
        log_info "権限設定完了"
    else
        # 非rootで実行中
        log_info "管理者権限が必要です"
        sudo setcap cap_net_raw,cap_net_admin=eip "$BINARY_PATH"
        log_info "権限設定完了"
    fi
    
    # 権限確認
    if getcap "$BINARY_PATH" | grep -q "cap_net_raw" && getcap "$BINARY_PATH" | grep -q "cap_net_admin"; then
        log_info "権限設定を確認しました"
    else
        log_error "権限設定に失敗しました"
        exit 1
    fi
}

# 監視を実行
run_monitor() {
    local interface="$1"
    
    if [ ! -f "$BINARY_PATH" ]; then
        log_error "バイナリが見つかりません。まずビルドしてください: $0 build"
        exit 1
    fi
    log_info "インターフェース '$interface' で TCP ウィンドウサイズ監視を開始します"
    log_info "停止するには Ctrl+C を押してください"
    
    # 権限確認
    if ! (getcap "$BINARY_PATH" | grep -q "cap_net_raw" && getcap "$BINARY_PATH" | grep -q "cap_net_admin"); then
        log_warn "権限が設定されていません。設定しますか？ (y/N)"
        read -r response
        if [[ "$response" =~ ^[Yy]$ ]]; then
            setup_permissions
        else
            log_info "root権限で実行してください: sudo $BINARY_PATH -i $interface"
            exit 1
        fi
    fi
    
    "$BINARY_PATH" -i "$interface" -v
}

# メイン処理
main() {
    case "${1:-help}" in
        "build")
            build_program
            ;;
        "setup")
            setup_permissions
            ;;
        "run")
            run_monitor "$2"
            ;;
        "help"|"--help"|"-h"|*)
            show_usage
            ;;
    esac
}

# スクリプト実行
main "$@"
