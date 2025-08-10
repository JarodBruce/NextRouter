#!/bin/bash
# Command line argument parsing for NextRouter

# Default values
DEFAULT_CONFIG_FILE="/etc/nextrouter/config.conf"

# Help text
show_help() {
    cat <<EOF
NextRouter - デュアルWANルーター セットアップスクリプト

使用法:
    $0 [オプション]

必須パラメーター:
    -w0, --wan0=<INTERFACE>     WAN0ネットワークインターフェース
    -w1, --wan1=<INTERFACE>     WAN1ネットワークインターフェース  
    -l0, --lan0=<INTERFACE>     LANネットワークインターフェース
    -lip, --local-ip=<IP/MASK>  LANローカルIPアドレス (例: 192.168.1.1/24)

オプションパラメーター:
    -c, --config=<FILE>         設定ファイルパス (デフォルト: $DEFAULT_CONFIG_FILE)
    --dry-run                   実際の変更を行わず、実行予定の内容のみ表示
    --debug                     デバッグモードで実行
    --backup-dir=<DIR>          バックアップディレクトリを指定
    --skip-prometheus           Prometheusのインストールをスキップ
    --skip-monitoring           監視ツールのインストールをスキップ
    --prometheus-version=<VER>  Prometheusバージョンを指定 (デフォルト: 2.45.0)

動作モード:
    --install                   フルインストール (デフォルト)
    --uninstall                 アンインストール
    --status                    現在の状態を表示
    --restart                   サービスを再起動
    --backup                    設定をバックアップ
    --restore=<BACKUP_DIR>      バックアップから復元

例:
    # 基本的なセットアップ
    $0 --wan0=eth0 --wan1=eth1 --lan0=eth2 --local-ip=192.168.1.1/24
    
    # ドライランでテスト
    $0 --wan0=eth0 --wan1=eth1 --lan0=eth2 --local-ip=192.168.1.1/24 --dry-run
    
    # デバッグモードで実行
    $0 --wan0=eth0 --wan1=eth1 --lan0=eth2 --local-ip=192.168.1.1/24 --debug
    
    # Prometheusをスキップ
    $0 --wan0=eth0 --wan1=eth1 --lan0=eth2 --local-ip=192.168.1.1/24 --skip-prometheus
    
    # 状態確認
    $0 --status
    
    # バックアップから復元
    $0 --restore=/var/backups/nextrouter/20250810_120000

EOF
}

# Parse command line arguments
parse_arguments() {
    local mode="install"
    local config_file="$DEFAULT_CONFIG_FILE"
    local skip_prometheus=false
    local skip_monitoring=false
    local backup_dir=""
    local restore_dir=""
    
    # Load existing config if available
    load_config "$config_file"
    
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --wan0=*|-w0=*)
                INTERFACES[WAN0]="${1#*=}"
                ;;
            --wan1=*|-w1=*)
                INTERFACES[WAN1]="${1#*=}"
                ;;
            --lan0=*|-l0=*)
                INTERFACES[LAN0]="${1#*=}"
                ;;
            --local-ip=*|-lip=*)
                INTERFACES[LAN_IP]="${1#*=}"
                ;;
            --config=*|-c=*)
                config_file="${1#*=}"
                load_config "$config_file"
                ;;
            --prometheus-version=*)
                CONFIG[PROMETHEUS_VERSION]="${1#*=}"
                ;;
            --backup-dir=*)
                backup_dir="${1#*=}"
                ;;
            --restore=*)
                mode="restore"
                restore_dir="${1#*=}"
                ;;
            --dry-run)
                CONFIG[DRY_RUN]="true"
                info "ドライランモードで実行します"
                ;;
            --debug)
                CONFIG[DEBUG_MODE]="true"
                info "デバッグモードで実行します"
                ;;
            --skip-prometheus)
                skip_prometheus=true
                ;;
            --skip-monitoring)
                skip_monitoring=true
                ;;
            --install)
                mode="install"
                ;;
            --uninstall)
                mode="uninstall"
                ;;
            --status)
                mode="status"
                ;;
            --restart)
                mode="restart"
                ;;
            --backup)
                mode="backup"
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            --version)
                echo "NextRouter v1.0.0"
                exit 0
                ;;
            *)
                error "不明なオプション: $1"
                echo "使用法については --help を参照してください"
                exit 1
                ;;
        esac
        shift
    done
    
    # Export mode and flags
    export OPERATION_MODE="$mode"
    export SKIP_PROMETHEUS="$skip_prometheus"
    export SKIP_MONITORING="$skip_monitoring"
    export BACKUP_DIR="$backup_dir"
    export RESTORE_DIR="$restore_dir"
    export CONFIG_FILE="$config_file"
    
    # Validate arguments based on mode
    case "$mode" in
        "install")
            validate_install_args
            ;;
        "restore")
            validate_restore_args
            ;;
        "status"|"restart"|"backup"|"uninstall")
            # These modes don't require interface arguments
            ;;
    esac
}

# Validate arguments for install mode
validate_install_args() {
    local missing_args=()
    
    # Check required arguments for install mode
    if [[ -z "${INTERFACES[WAN0]}" ]]; then
        missing_args+=("--wan0")
    fi
    
    if [[ -z "${INTERFACES[WAN1]}" ]]; then
        missing_args+=("--wan1")
    fi
    
    if [[ -z "${INTERFACES[LAN0]}" ]]; then
        missing_args+=("--lan0")
    fi
    
    if [[ -z "${INTERFACES[LAN_IP]}" ]]; then
        missing_args+=("--local-ip")
    fi
    
    if [[ ${#missing_args[@]} -gt 0 ]]; then
        error "以下の必須引数が不足しています: ${missing_args[*]}"
        echo
        echo "使用法については --help を参照してください"
        exit 1
    fi
    
    # Validate configuration
    if ! validate_config; then
        error "設定の検証に失敗しました"
        exit 1
    fi
}

# Validate arguments for restore mode
validate_restore_args() {
    if [[ -z "$RESTORE_DIR" ]]; then
        error "復元モードではバックアップディレクトリの指定が必要です"
        exit 1
    fi
    
    if [[ ! -d "$RESTORE_DIR" ]]; then
        error "指定されたバックアップディレクトリが存在しません: $RESTORE_DIR"
        exit 1
    fi
}

# Display current configuration
show_config() {
    echo "=== 現在の設定 ==="
    echo "動作モード: $OPERATION_MODE"
    echo "WAN0: ${INTERFACES[WAN0]}"
    echo "WAN1: ${INTERFACES[WAN1]}"
    echo "LAN0: ${INTERFACES[LAN0]}"
    echo "LAN IP: ${INTERFACES[LAN_IP]}"
    echo "設定ファイル: $CONFIG_FILE"
    echo "ドライラン: ${CONFIG[DRY_RUN]}"
    echo "デバッグ: ${CONFIG[DEBUG_MODE]}"
    echo "Prometheusスキップ: $SKIP_PROMETHEUS"
    echo "監視ツールスキップ: $SKIP_MONITORING"
    echo "=================="
}
