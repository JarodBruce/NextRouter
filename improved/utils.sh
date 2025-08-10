#!/bin/bash
# 共通のユーティリティ関数

# 出力色設定
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # ノーカラー

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

# ipcalcが利用できない場合にネットワーク詳細を手動で計算する関数
calculate_network_manual() {
    local ip_cidr="$1"
    local ip
    ip=$(echo "$ip_cidr" | cut -d'/' -f1)
    local prefix
    prefix=$(echo "$ip_cidr" | cut -d'/' -f2)
    
    # IPを10進数に変換
    local IFS='.'
    local ip_array
    ip_array=($ip)
    local ip_decimal
    ip_decimal=$((${ip_array[0]} * 256**3 + ${ip_array[1]} * 256**2 + ${ip_array[2]} * 256 + ${ip_array[3]}))
    
    # ネットワークマスクを計算
    local mask_decimal
    mask_decimal=$(( (0xFFFFFFFF << (32 - prefix)) & 0xFFFFFFFF ))
    
    # ネットワークアドレスを計算
    local network_decimal
    network_decimal=$((ip_decimal & mask_decimal))
    
    # ドット区切りの10進数に戻す
    local network_addr
    network_addr="$((network_decimal >> 24)).$((network_decimal >> 16 & 255)).$((network_decimal >> 8 & 255)).$((network_decimal & 255))"
    
    # ブロードキャストを計算
    local broadcast_decimal
    broadcast_decimal=$((network_decimal | (0xFFFFFFFF >> prefix)))
    local broadcast_addr
    broadcast_addr="$((broadcast_decimal >> 24)).$((broadcast_decimal >> 16 & 255)).$((broadcast_decimal >> 8 & 255)).$((broadcast_decimal & 255))"
    
    # ネットマスクを計算
    local netmask
    netmask="$((mask_decimal >> 24)).$((mask_decimal >> 16 & 255)).$((mask_decimal >> 8 & 255)).$((mask_decimal & 255))"
    
    echo "NETWORK:$network_addr/$prefix"
    echo "NETMASK:$netmask"
    echo "BROADCAST:$broadcast_addr"
}

# ルート権限チェック
check_root() {
    if [[ $EUID -ne 0 ]]; then
        error "このスクリプトはルートとして実行する必要があります。sudoを使用してください。"
        exit 1
    fi
}
