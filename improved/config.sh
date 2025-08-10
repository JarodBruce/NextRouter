#!/bin/bash
# Configuration management for NextRouter

# Global configuration
declare -gA CONFIG=(
    [PROMETHEUS_VERSION]="2.45.0"
    [PROMETHEUS_USER]="prometheus"
    [PROMETHEUS_HOME]="/opt/prometheus"
    [PROMETHEUS_DATA]="/var/lib/prometheus"
    [PROMETHEUS_CONFIG]="/etc/prometheus"
    
    [DHCP_RANGE_START]=".100"
    [DHCP_RANGE_END]=".200"
    [DNS_SERVER]="1.1.1.1"
    
    [LOG_LEVEL]="INFO"
    [DEBUG_MODE]="false"
    [DRY_RUN]="false"
)

# Network interface configuration
declare -gA INTERFACES=(
    [WAN0]=""
    [WAN1]=""
    [LAN0]=""
    [LAN_IP]=""
)

# Runtime variables
declare -gA RUNTIME=(
    [WAN1_IP]=""
    [WAN2_IP]=""
    [WAN1_GW]=""
    [WAN2_GW]=""
    [LAN_NETWORK]=""
    [LAN_NETMASK]=""
    [LAN_BROADCAST]=""
    [LAN_GATEWAY]=""
)

# Load configuration from file if exists
load_config() {
    local config_file="${1:-/etc/nextrouter/config.conf}"
    
    if [[ -f "$config_file" ]]; then
        source "$config_file"
        log "設定ファイルを読み込みました: $config_file"
    fi
}

# Save current configuration
save_config() {
    local config_file="${1:-/etc/nextrouter/config.conf}"
    local config_dir=$(dirname "$config_file")
    
    sudo mkdir -p "$config_dir"
    
    cat > /tmp/nextrouter_config <<EOF
# NextRouter Configuration
# Generated on $(date)

# Network Interfaces
INTERFACES[WAN0]="${INTERFACES[WAN0]}"
INTERFACES[WAN1]="${INTERFACES[WAN1]}"
INTERFACES[LAN0]="${INTERFACES[LAN0]}"
INTERFACES[LAN_IP]="${INTERFACES[LAN_IP]}"

# Prometheus Configuration  
CONFIG[PROMETHEUS_VERSION]="${CONFIG[PROMETHEUS_VERSION]}"
CONFIG[PROMETHEUS_USER]="${CONFIG[PROMETHEUS_USER]}"

# DHCP Configuration
CONFIG[DHCP_RANGE_START]="${CONFIG[DHCP_RANGE_START]}"
CONFIG[DHCP_RANGE_END]="${CONFIG[DHCP_RANGE_END]}"
CONFIG[DNS_SERVER]="${CONFIG[DNS_SERVER]}"
EOF

    sudo mv /tmp/nextrouter_config "$config_file"
    sudo chmod 644 "$config_file"
    log "設定を保存しました: $config_file"
}

# Validate configuration
validate_config() {
    local errors=0
    
    # Validate required interfaces
    for interface in WAN0 WAN1 LAN0 LAN_IP; do
        if [[ -z "${INTERFACES[$interface]}" ]]; then
            error "必須設定が不足: $interface"
            ((errors++))
        fi
    done
    
    # Validate IP format
    if [[ -n "${INTERFACES[LAN_IP]}" ]]; then
        if ! [[ "${INTERFACES[LAN_IP]}" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+/[0-9]+$ ]]; then
            error "不正なIP形式: ${INTERFACES[LAN_IP]}"
            ((errors++))
        fi
    fi
    
    # Validate interface existence
    for interface in WAN0 WAN1 LAN0; do
        if [[ -n "${INTERFACES[$interface]}" ]]; then
            if ! ip link show "${INTERFACES[$interface]}" &>/dev/null; then
                warning "ネットワークインターフェースが見つかりません: ${INTERFACES[$interface]}"
            fi
        fi
    done
    
    return $errors
}
