#!/bin/bash

# Prometheus管理ユーティリティスクリプト

set -e

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

# 使用方法
show_usage() {
    echo "Prometheus管理ユーティリティ"
    echo ""
    echo "使用方法: $0 [オプション]"
    echo ""
    echo "オプション:"
    echo "  status    - Prometheusのステータスを表示"
    echo "  start     - Prometheusを起動"
    echo "  stop      - Prometheusを停止"
    echo "  restart   - Prometheusを再起動"
    echo "  reload    - 設定をリロード"
    echo "  check     - 設定ファイルを検証"
    echo "  logs      - ログを表示"
    echo "  test      - 接続テスト"
    echo "  uninstall - Prometheusをアンインストール"
    echo "  help      - このヘルプを表示"
}

# ステータス確認
check_status() {
    echo "=== Prometheus ステータス ==="
    if systemctl is-active --quiet prometheus; then
        success "Prometheus は稼働中です"
        systemctl status prometheus --no-pager -l | head -10
    else
        warning "Prometheus は停止中です"
    fi
    
    echo ""
    echo "=== 接続テスト ==="
    if curl -s http://localhost:9090/-/healthy > /dev/null; then
        success "Prometheus API に接続できます"
        info "Webインターフェース: http://localhost:9090"
    else
        warning "Prometheus API に接続できません"
    fi
}

# 起動
start_service() {
    info "Prometheusを起動しています..."
    sudo systemctl start prometheus
    sleep 2
    if systemctl is-active --quiet prometheus; then
        success "Prometheusが起動しました"
    else
        error "Prometheusの起動に失敗しました"
    fi
}

# 停止
stop_service() {
    info "Prometheusを停止しています..."
    sudo systemctl stop prometheus
    sleep 2
    if ! systemctl is-active --quiet prometheus; then
        success "Prometheusが停止しました"
    else
        error "Prometheusの停止に失敗しました"
    fi
}

# 再起動
restart_service() {
    info "Prometheusを再起動しています..."
    sudo systemctl restart prometheus
    sleep 3
    if systemctl is-active --quiet prometheus; then
        success "Prometheusが再起動しました"
    else
        error "Prometheusの再起動に失敗しました"
    fi
}

# 設定リロード
reload_config() {
    info "設定をリロードしています..."
    if curl -s -X POST http://localhost:9090/-/reload; then
        success "設定のリロードが完了しました"
    else
        error "設定のリロードに失敗しました"
        warning "Prometheusが稼働していることを確認してください"
    fi
}

# 設定チェック
check_config() {
    info "設定ファイルを検証しています..."
    if promtool check config /etc/prometheus/prometheus.yml; then
        success "設定ファイルは正常です"
    else
        error "設定ファイルにエラーがあります"
    fi
    
    # ルールファイルもチェック
    if ls /etc/prometheus/rules/*.yml 1> /dev/null 2>&1; then
        info "ルールファイルを検証しています..."
        promtool check rules /etc/prometheus/rules/*.yml
    fi
}

# ログ表示
show_logs() {
    info "Prometheusのログを表示します (Ctrl+C で終了)"
    sudo journalctl -u prometheus -f
}

# 接続テスト
test_connection() {
    echo "=== Prometheus 接続テスト ==="
    
    # ヘルスチェック
    if curl -s http://localhost:9090/-/healthy > /dev/null; then
        success "ヘルスチェック: OK"
    else
        error "ヘルスチェック: 失敗"
    fi
    
    # API テスト
    if curl -s http://localhost:9090/api/v1/status/config > /dev/null; then
        success "API接続: OK"
    else
        warning "API接続: 失敗"
    fi
    
    # ターゲット状態
    info "ターゲット状態を確認中..."
    targets=$(curl -s http://localhost:9090/api/v1/targets | jq -r '.data.activeTargets[].labels.job' 2>/dev/null | sort | uniq)
    if [ -n "$targets" ]; then
        success "アクティブなターゲット:"
        echo "$targets" | while read -r target; do
            echo "  - $target"
        done
    else
        warning "ターゲット情報を取得できませんでした"
    fi
}

# アンインストール
uninstall() {
    warning "Prometheusをアンインストールします"
    read -p "続行しますか? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        info "アンインストールをキャンセルしました"
        return
    fi
    
    info "Prometheusサービスを停止しています..."
    sudo systemctl stop prometheus 2>/dev/null || true
    sudo systemctl disable prometheus 2>/dev/null || true
    
    info "ファイルを削除しています..."
    sudo rm -rf /opt/prometheus
    sudo rm -rf /var/lib/prometheus
    sudo rm -rf /etc/prometheus
    sudo rm -f /etc/systemd/system/prometheus.service
    sudo rm -f /usr/local/bin/prometheus
    sudo rm -f /usr/local/bin/promtool
    
    info "ユーザーを削除しています..."
    sudo userdel prometheus 2>/dev/null || true
    
    info "systemd設定をリロードしています..."
    sudo systemctl daemon-reload
    
    success "Prometheusのアンインストールが完了しました"
}

# メイン処理
case "${1:-help}" in
    "status")
        check_status
        ;;
    "start")
        start_service
        ;;
    "stop")
        stop_service
        ;;
    "restart")
        restart_service
        ;;
    "reload")
        reload_config
        ;;
    "check")
        check_config
        ;;
    "logs")
        show_logs
        ;;
    "test")
        test_connection
        ;;
    "uninstall")
        uninstall
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
