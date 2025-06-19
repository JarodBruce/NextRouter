#!/bin/bash

# Prometheus インストール・セットアップスクリプト
# prometheus.yml 設定ファイルを使用してPrometheusをインストール・設定します

set -e  # エラー時に終了

# 設定
PROMETHEUS_VERSION="2.45.0"
PROMETHEUS_USER="prometheus"
PROMETHEUS_HOME="/opt/prometheus"
PROMETHEUS_DATA="/var/lib/prometheus"
PROMETHEUS_CONFIG="/etc/prometheus"
WORKING_DIR=$(pwd)
PROMETHEUS_YML_FILE="$WORKING_DIR/prometheus.yml"

# 出力用の色設定
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # 色リセット

# ログ出力関数
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}"
}

info() {
    echo -e "${BLUE}[INFO] $1${NC}"
}

error() {
    echo -e "${RED}[ERROR] $1${NC}" >&2
}

warning() {
    echo -e "${YELLOW}[WARNING] $1${NC}"
}

success() {
    echo -e "${GREEN}[SUCCESS] $1${NC}"
}

# rootユーザーチェック
check_root() {
    if [[ $EUID -eq 0 ]]; then
        error "セキュリティ上の理由により、このスクリプトをrootユーザーで実行しないでください"
        error "sudo権限を持つ一般ユーザーで実行してください"
        exit 1
    fi
}

# prometheus.yml ファイルの存在確認
check_prometheus_yml() {
    log "prometheus.yml ファイルをチェックしています..."
    
    if [[ ! -f "$PROMETHEUS_YML_FILE" ]]; then
        error "prometheus.yml ファイルが見つかりません: $PROMETHEUS_YML_FILE"
        error "現在のディレクトリにprometheus.ymlファイルを配置してください"
        exit 1
    fi
    
    # YAML構文の基本チェック
    if command -v python3 &> /dev/null; then
        if ! python3 -c "import yaml; yaml.safe_load(open('$PROMETHEUS_YML_FILE'))" 2>/dev/null; then
            warning "prometheus.yml の構文に問題がある可能性があります"
        else
            success "prometheus.yml ファイルが見つかりました"
        fi
    else
        info "prometheus.yml ファイルが見つかりました (詳細な構文チェックはPrometheus起動時に実行されます)"
    fi
}

# sudo権限チェック
check_sudo() {
    if ! command -v sudo &> /dev/null; then
        error "sudoコマンドが必要ですがインストールされていません"
        exit 1
    fi
    
    if ! sudo -v; then
        error "sudo権限が必要です"
        exit 1
    fi
}

# システム要件チェック
check_requirements() {
    log "システム要件をチェックしています..."
    
    # OS確認
    if [[ "$OSTYPE" != "linux-gnu"* ]]; then
        error "このスクリプトはLinuxシステム向けです"
        exit 1
    fi
    
    # アーキテクチャ確認
    ARCH=$(uname -m)
    case $ARCH in
        x86_64)
            PROMETHEUS_ARCH="amd64"
            ;;
        aarch64)
            PROMETHEUS_ARCH="arm64"
            ;;
        *)
            error "サポートされていないアーキテクチャです: $ARCH"
            exit 1
            ;;
    esac
    
    # 必要なコマンドの確認
    local required_commands=("wget" "tar" "systemctl")
    for cmd in "${required_commands[@]}"; do
        if ! command -v $cmd &> /dev/null; then
            error "必要なコマンドがインストールされていません: $cmd"
            exit 1
        fi
    done
    
    success "システム要件チェック完了 (アーキテクチャ: $PROMETHEUS_ARCH)"
}

# prometheusユーザー作成
create_user() {
    log "prometheusユーザーを作成しています..."
    
    if id "$PROMETHEUS_USER" &>/dev/null; then
        warning "ユーザー $PROMETHEUS_USER は既に存在します"
    else
        sudo useradd --no-create-home --shell /bin/false $PROMETHEUS_USER
        success "ユーザー $PROMETHEUS_USER を作成しました"
    fi
}

# ディレクトリ作成
create_directories() {
    log "必要なディレクトリを作成しています..."
    
    local directories=("$PROMETHEUS_HOME" "$PROMETHEUS_DATA" "$PROMETHEUS_CONFIG" "$PROMETHEUS_CONFIG/rules")
    
    for dir in "${directories[@]}"; do
        sudo mkdir -p "$dir"
        sudo chown $PROMETHEUS_USER:$PROMETHEUS_USER "$dir"
    done
    
    success "ディレクトリを作成し、権限を設定しました"
}

# Prometheusダウンロード・インストール
download_prometheus() {
    log "Prometheus v$PROMETHEUS_VERSION をダウンロードしています..."
    
    PROMETHEUS_TARBALL="prometheus-$PROMETHEUS_VERSION.linux-$PROMETHEUS_ARCH.tar.gz"
    DOWNLOAD_URL="https://github.com/prometheus/prometheus/releases/download/v$PROMETHEUS_VERSION/$PROMETHEUS_TARBALL"
    
    cd /tmp
    if ! wget -q "$DOWNLOAD_URL"; then
        error "Prometheusのダウンロードに失敗しました"
        exit 1
    fi
    
    if [[ ! -f "$PROMETHEUS_TARBALL" ]]; then
        error "ダウンロードファイルが見つかりません"
        exit 1
    fi
    
    log "Prometheusを展開しています..."
    tar xf "$PROMETHEUS_TARBALL"
    
    PROMETHEUS_DIR="prometheus-$PROMETHEUS_VERSION.linux-$PROMETHEUS_ARCH"
    
    # バイナリファイルをコピー
    info "バイナリファイルをコピーしています..."
    sudo cp "$PROMETHEUS_DIR/prometheus" $PROMETHEUS_HOME/
    sudo cp "$PROMETHEUS_DIR/promtool" $PROMETHEUS_HOME/
    
    # コンソールファイルをコピー
    info "コンソールファイルをコピーしています..."
    sudo cp -r "$PROMETHEUS_DIR/consoles" $PROMETHEUS_CONFIG/
    sudo cp -r "$PROMETHEUS_DIR/console_libraries" $PROMETHEUS_CONFIG/
    
    # 権限設定
    sudo chown $PROMETHEUS_USER:$PROMETHEUS_USER $PROMETHEUS_HOME/prometheus
    sudo chown $PROMETHEUS_USER:$PROMETHEUS_USER $PROMETHEUS_HOME/promtool
    sudo chown -R $PROMETHEUS_USER:$PROMETHEUS_USER $PROMETHEUS_CONFIG/consoles
    sudo chown -R $PROMETHEUS_USER:$PROMETHEUS_USER $PROMETHEUS_CONFIG/console_libraries
    
    # 簡単にアクセスできるようにシンボリックリンクを作成
    sudo ln -sf $PROMETHEUS_HOME/prometheus /usr/local/bin/prometheus
    sudo ln -sf $PROMETHEUS_HOME/promtool /usr/local/bin/promtool
    
    # 一時ファイルを削除
    rm -rf /tmp/"$PROMETHEUS_DIR" /tmp/"$PROMETHEUS_TARBALL"
    
    success "Prometheus インストール完了"
}

# 設定ファイルのセットアップ
setup_config() {
    log "Prometheus設定ファイルをセットアップしています..."
    
    # 既存のprometheus.ymlを使用
    if [[ -f "$PROMETHEUS_YML_FILE" ]]; then
        info "現在のディレクトリのprometheus.ymlを使用します"
        sudo cp "$PROMETHEUS_YML_FILE" $PROMETHEUS_CONFIG/
        sudo chown $PROMETHEUS_USER:$PROMETHEUS_USER $PROMETHEUS_CONFIG/prometheus.yml
        success "prometheus.yml を $PROMETHEUS_CONFIG/ にコピーしました"
    else
        warning "prometheus.ymlが見つかりません。デフォルト設定を作成します"
        create_default_config
    fi
    
    # アラートルールファイルが存在する場合はコピー
    if [[ -f "$WORKING_DIR/alert_rules.yml" ]]; then
        sudo cp "$WORKING_DIR/alert_rules.yml" $PROMETHEUS_CONFIG/rules/
        sudo chown $PROMETHEUS_USER:$PROMETHEUS_USER $PROMETHEUS_CONFIG/rules/alert_rules.yml
        info "alert_rules.yml を $PROMETHEUS_CONFIG/rules/ にコピーしました"
    fi
    
    # 設定ファイルの検証
    log "設定ファイルを検証しています..."
    if sudo -u $PROMETHEUS_USER $PROMETHEUS_HOME/promtool check config $PROMETHEUS_CONFIG/prometheus.yml; then
        success "Prometheus設定ファイルの検証が完了しました"
    else
        error "Prometheus設定ファイルの検証に失敗しました"
        error "prometheus.ymlの内容を確認してください"
        exit 1
    fi
}

# デフォルト設定ファイル作成（prometheus.ymlが存在しない場合）
create_default_config() {
    log "デフォルトのPrometheus設定ファイルを作成しています..."
    
    sudo tee $PROMETHEUS_CONFIG/prometheus.yml > /dev/null <<EOF
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "rules/*.yml"

scrape_configs:
  - job_name: 'prometheus'
    static_configs:
      - targets: ['localhost:9090']

  - job_name: 'rust-app'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
    scrape_interval: 5s
    
  - job_name: 'node-exporter'
    static_configs:
      - targets: ['localhost:9100']
    scrape_interval: 5s
EOF
    
    sudo chown $PROMETHEUS_USER:$PROMETHEUS_USER $PROMETHEUS_CONFIG/prometheus.yml
    info "デフォルト設定ファイルを作成しました"
}

# systemdサービス作成
create_service() {
    log "systemdサービスを作成しています..."
    
    sudo tee /etc/systemd/system/prometheus.service > /dev/null <<EOF
[Unit]
Description=Prometheus Time Series Collection and Processing Server
Documentation=https://prometheus.io/
Wants=network-online.target
After=network-online.target

[Service]
User=$PROMETHEUS_USER
Group=$PROMETHEUS_USER
Type=simple
Restart=always
RestartSec=10
StartLimitInterval=0

ExecStart=$PROMETHEUS_HOME/prometheus \\
    --config.file=$PROMETHEUS_CONFIG/prometheus.yml \\
    --storage.tsdb.path=$PROMETHEUS_DATA \\
    --storage.tsdb.retention.time=30d \\
    --web.console.templates=$PROMETHEUS_CONFIG/consoles \\
    --web.console.libraries=$PROMETHEUS_CONFIG/console_libraries \\
    --web.listen-address=0.0.0.0:9090 \\
    --web.enable-lifecycle \\
    --web.enable-admin-api

[Install]
WantedBy=multi-user.target
EOF
    
    sudo systemctl daemon-reload
    sudo systemctl enable prometheus
    
    success "systemdサービスを作成し、有効化しました"
}

# Prometheus起動
start_prometheus() {
    log "Prometheusを起動しています..."
    
    sudo systemctl start prometheus
    
    # 起動を少し待つ
    sleep 5
    
    if sudo systemctl is-active --quiet prometheus; then
        success "Prometheusの起動に成功しました"
        info "Prometheus Webインターフェース: http://localhost:9090"
        
        # 追加の起動確認
        if command -v curl &> /dev/null; then
            log "API接続をテストしています..."
            if curl -s http://localhost:9090/-/healthy > /dev/null; then
                success "Prometheus APIが正常に応答しています"
            else
                warning "Prometheus APIに接続できませんでした（起動中の可能性があります）"
            fi
        fi
    else
        error "Prometheusの起動に失敗しました"
        error "ログを確認してください: sudo journalctl -u prometheus -f"
        exit 1
    fi
}

# ステータス表示
show_status() {
    echo
    success "=== Prometheus セットアップ完了! ==="
    echo
    echo "=== サービス状態 ==="
    sudo systemctl status prometheus --no-pager -l | head -15
    echo
    echo "=== アクセス情報 ==="
    echo "Webインターフェース: http://localhost:9090"
    echo "API エンドポイント:   http://localhost:9090/api/v1/"
    echo "ヘルスチェック:      http://localhost:9090/-/healthy"
    echo "設定リロード:        curl -X POST http://localhost:9090/-/reload"
    echo
    echo "=== 設定ファイル ==="
    echo "設定ファイル:        $PROMETHEUS_CONFIG/prometheus.yml"
    echo "データディレクトリ:  $PROMETHEUS_DATA"
    echo "ルールディレクトリ:  $PROMETHEUS_CONFIG/rules"
    echo
    echo "=== サービス管理コマンド ==="
    echo "起動:   sudo systemctl start prometheus"
    echo "停止:   sudo systemctl stop prometheus"
    echo "再起動: sudo systemctl restart prometheus"
    echo "状態:   sudo systemctl status prometheus"
    echo "ログ:   sudo journalctl -u prometheus -f"
    echo
    echo "=== 設定検証 ==="
    echo "設定チェック: promtool check config $PROMETHEUS_CONFIG/prometheus.yml"
    echo "ルールチェック: promtool check rules $PROMETHEUS_CONFIG/rules/*.yml"
    echo
    echo "=== 便利なクエリ例 ==="
    echo "- up: 全てのターゲットの稼働状態"
    echo "- prometheus_config_last_reload_successful: 設定リロード成功状態"
    echo "- rate(prometheus_http_requests_total[5m]): HTTP リクエスト率"
    
    # 設定内容の表示
    if [[ -f "$PROMETHEUS_CONFIG/prometheus.yml" ]]; then
        echo
        echo "=== 現在の設定内容 ==="
        echo "Job設定:"
        grep -A 5 "job_name:" "$PROMETHEUS_CONFIG/prometheus.yml" | head -20
    fi
}

# メイン処理
main() {
    echo
    echo "=============================================="
    echo "    Prometheus インストール・セットアップ"
    echo "=============================================="
    echo
    
    log "Prometheusセットアップを開始します..."
    
    check_root
    check_sudo
    check_prometheus_yml
    check_requirements
    create_user
    create_directories
    download_prometheus
    setup_config
    create_service
    start_prometheus
    show_status
    
    echo
    success "Prometheus セットアップが正常に完了しました!"
    info "ブラウザで http://localhost:9090 にアクセスしてPrometheusを確認できます"
}

# エラーハンドリング
trap 'error "スクリプト実行中にエラーが発生しました"; exit 1' ERR

# メイン処理実行
main "$@"