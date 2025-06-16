#!/bin/bash

# nftablesの設定スクリプト
# 使用方法: ./nftables-setup.sh [WAN_INTERFACE] [LAN_INTERFACE]
# 例: ./nftables-setup.sh eth0 eth1

# === インターフェース設定 ===
# コマンドライン引数があれば使用、なければデフォルト値
WAN_INTERFACE="${1:-ens18}"     # WAN (外部ネットワーク) - デフォルト: ens18
LAN_INTERFACE="${2:-ens19}"     # LAN (内部ネットワーク) - デフォルト: ens19

# === ネットワーク設定 ===
LAN_IPV4_NETWORK="10.40.0.0/24"        # LANのIPv4ネットワーク
LAN_IPV4_GATEWAY="10.40.0.1/24"        # LANのIPv4ゲートウェイ
LAN_IPV6_NETWORK="fd00:40::/64"         # LANのIPv6ネットワーク
LAN_IPV6_GATEWAY="fd00:40::1/64"        # LANのIPv6ゲートウェイ
DHCP_IPV4_START="10.40.0.100"          # DHCP IPv4開始アドレス
DHCP_IPV4_END="10.40.0.200"            # DHCP IPv4終了アドレス
DHCP_IPV6_START="fd00:40::100"         # DHCP IPv6開始アドレス
DHCP_IPV6_END="fd00:40::200"           # DHCP IPv6終了アドレス

echo "=== nftables設定スクリプト ==="
echo "WAN Interface: $WAN_INTERFACE"
echo "LAN Interface: $LAN_INTERFACE"
echo "LAN IPv4: $LAN_IPV4_NETWORK (Gateway: ${LAN_IPV4_GATEWAY%/*})"
echo "LAN IPv6: $LAN_IPV6_NETWORK (Gateway: ${LAN_IPV6_GATEWAY%/*})"
echo ""

# インターフェースの存在確認
if ! ip link show "$WAN_INTERFACE" &>/dev/null; then
    echo "エラー: WAN interface '$WAN_INTERFACE' が見つかりません"
    echo "利用可能なインターフェース:"
    ip link show | grep -E '^[0-9]+:' | awk -F': ' '{print "  " $2}' | sed 's/@.*//'
    exit 1
fi

if ! ip link show "$LAN_INTERFACE" &>/dev/null; then
    echo "エラー: LAN interface '$LAN_INTERFACE' が見つかりません"
    echo "利用可能なインターフェース:"
    ip link show | grep -E '^[0-9]+:' | awk -F': ' '{print "  " $2}' | sed 's/@.*//'
    exit 1
fi

if ! command -v nftables &> /dev/null; then
    apt update && apt install -y nftables
fi

# dnsmasqをインストール（DHCPv6サーバー用）
if ! command -v dnsmasq &> /dev/null; then
    apt update && apt install -y dnsmasq
fi

# 既存のルールをクリア
nft flush ruleset

# LANインターフェースにIPアドレスを設定
# まず既存のIPアドレスをクリア
ip addr flush dev "$LAN_INTERFACE" 2>/dev/null || true
ip addr add "$LAN_IPV4_GATEWAY" dev "$LAN_INTERFACE"
# IPv6はDHCPv6サーバー用にULA（Unique Local Address）を設定
ip addr add "$LAN_IPV6_GATEWAY" dev "$LAN_INTERFACE"
ip link set "$LAN_INTERFACE" up

# IP転送を有効化（重要）
echo 1 > /proc/sys/net/ipv4/ip_forward
# IPv6転送を有効化（DHCPv6サーバー用）
echo 1 > /proc/sys/net/ipv6/conf/all/forwarding
# IPv6 Router Advertisementを有効化
echo 1 > "/proc/sys/net/ipv6/conf/$LAN_INTERFACE/accept_ra"
echo 2 > "/proc/sys/net/ipv6/conf/$LAN_INTERFACE/accept_ra_rt_info_max_plen"

# テーブルとチェーンの作成
nft add table inet filter
nft add table inet nat

# フィルタテーブルのチェーン作成
nft add chain inet filter input { type filter hook input priority 0\; policy drop\; }
nft add chain inet filter forward { type filter hook forward priority 0\; policy drop\; }
nft add chain inet filter output { type filter hook output priority 0\; policy accept\; }

# NATテーブルのチェーン作成
nft add chain inet nat prerouting { type nat hook prerouting priority -100\; }
nft add chain inet nat postrouting { type nat hook postrouting priority 100\; }

# ローカルループバックを許可
nft add rule inet filter input iif lo accept
nft add rule inet filter output oif lo accept

# 確立済み・関連する接続を許可
nft add rule inet filter input ct state established,related accept
nft add rule inet filter forward ct state established,related accept

# LAN(${LAN_INTERFACE})からの入力を許可
nft add rule inet filter input iif "$LAN_INTERFACE" accept

# LAN(${LAN_INTERFACE})からWAN(${WAN_INTERFACE})への転送を許可（重要）
nft add rule inet filter forward iif "$LAN_INTERFACE" oif "$WAN_INTERFACE" accept
# WAN(${WAN_INTERFACE})からLAN(${LAN_INTERFACE})への応答も許可（重要）
nft add rule inet filter forward iif "$WAN_INTERFACE" oif "$LAN_INTERFACE" ct state established,related accept
# IPv6パススルー用：IPv6トラフィックの双方向転送を許可
nft add rule inet filter forward iif "$LAN_INTERFACE" oif "$WAN_INTERFACE" ip6 version 6 accept
nft add rule inet filter forward iif "$WAN_INTERFACE" oif "$LAN_INTERFACE" ip6 version 6 accept

# SSH接続を許可 (必要に応じて)
nft add rule inet filter input tcp dport 22 accept

# DHCP(DNSmasq等)を許可 (LANから)
nft add rule inet filter input iif "$LAN_INTERFACE" udp dport 67 accept

# DHCPv6サーバーポートを許可
nft add rule inet filter input iif "$LAN_INTERFACE" udp dport 547 accept
# DHCPv6クライアントポートも許可
nft add rule inet filter input iif "$LAN_INTERFACE" udp dport 546 accept

# DNS転送を許可
nft add rule inet filter input iif "$LAN_INTERFACE" udp dport 53 accept
nft add rule inet filter input iif "$LAN_INTERFACE" tcp dport 53 accept

# ICMP(ping)を許可
nft add rule inet filter input icmp type echo-request accept
nft add rule inet filter forward icmp type echo-request accept
nft add rule inet filter forward icmp type echo-reply accept

# ICMPv6も許可（IPv6で重要）
nft add rule inet filter input icmpv6 type echo-request accept
nft add rule inet filter forward icmpv6 type echo-request accept
nft add rule inet filter forward icmpv6 type echo-reply accept
# IPv6で必要なICMPv6メッセージを許可
nft add rule inet filter input icmpv6 type { nd-neighbor-solicit, nd-neighbor-advert, nd-router-solicit, nd-router-advert } accept
nft add rule inet filter forward icmpv6 type { nd-neighbor-solicit, nd-neighbor-advert, nd-router-solicit, nd-router-advert } accept

# NAT設定 - LANからWANへのマスカレード（重要）
nft add rule inet nat postrouting oif "$WAN_INTERFACE" ip saddr "${LAN_IPV4_NETWORK}" masquerade
# IPv6はパススルー（ブリッジ）のためNATしない

# LANネットワーク用の追加設定
nft add rule inet filter input ip saddr "${LAN_IPV4_NETWORK}" accept
nft add rule inet filter forward ip saddr "${LAN_IPV4_NETWORK}" accept

# IPv6パススルー用の設定（特定のネットワークに限定せず全IPv6を許可）
nft add rule inet filter forward ip6 version 6 accept

# LANのIPv6ネットワーク用の設定（DHCPv6用）
nft add rule inet filter input ip6 saddr "${LAN_IPV6_NETWORK}" accept
nft add rule inet filter forward ip6 saddr "${LAN_IPV6_NETWORK}" accept

# 設定を保存
nft list ruleset > /etc/nftables.conf

# systemd-resolvedのDNSスタブリスナーを無効化
mkdir -p /etc/systemd/resolved.conf.d
cat > /etc/systemd/resolved.conf.d/dns.conf << EOF
[Resolve]
DNSStubListener=no
EOF

# systemd-resolvedを再起動
systemctl restart systemd-resolved

# 既存のresolv.confを削除してsystemd-resolved以外のDNSを使用
rm -f /etc/resolv.conf
cat > /etc/resolv.conf << EOF
nameserver 1.1.1.1
EOF

# IP転送を永続化
echo 'net.ipv4.ip_forward=1' >> /etc/sysctl.conf
# IPv6転送を永続化（DHCPv6サーバー用）
echo 'net.ipv6.conf.all.forwarding=1' >> /etc/sysctl.conf
echo "net.ipv6.conf.$LAN_INTERFACE.accept_ra=1" >> /etc/sysctl.conf
echo "net.ipv6.conf.$LAN_INTERFACE.accept_ra_rt_info_max_plen=2" >> /etc/sysctl.conf
sysctl -p

# dnsmasqの設定ファイルを作成
cat > /etc/dnsmasq.conf << EOF
# 基本設定
interface=$LAN_INTERFACE
bind-interfaces
domain-needed
bogus-priv

# DNS設定
server=1.1.1.1
server=8.8.8.8
server=2606:4700:4700::1111
server=2001:4860:4860::8888

# IPv4 DHCP設定
dhcp-range=$DHCP_IPV4_START,$DHCP_IPV4_END,255.255.255.0,24h
dhcp-option=option:router,${LAN_IPV4_GATEWAY%/*}
dhcp-option=option:dns-server,${LAN_IPV4_GATEWAY%/*}

# IPv6設定
enable-ra
dhcp-range=$DHCP_IPV6_START,$DHCP_IPV6_END,64,24h
dhcp-option=option6:dns-server,[${LAN_IPV6_GATEWAY%/*}]

# RA (Router Advertisement) 設定
ra-param=$LAN_INTERFACE,60,1800

# ログ設定
log-dhcp
log-queries

# キャッシュサイズ
cache-size=1000
EOF

# dnsmasqサービスを有効化・開始
systemctl enable dnsmasq
systemctl restart dnsmasq

echo "nftables設定が完了しました"
echo "WAN: $WAN_INTERFACE"
echo "LAN: $LAN_INTERFACE ($LAN_IPV4_NETWORK, $LAN_IPV6_NETWORK)"
echo "IPv4ゲートウェイ: ${LAN_IPV4_GATEWAY%/*}"
echo "IPv6ゲートウェイ: ${LAN_IPV6_GATEWAY%/*}"
echo ""
echo "DHCP設定:"
echo "  IPv4: $DHCP_IPV4_START-$DHCP_IPV4_END"
echo "  IPv6: $DHCP_IPV6_START-$DHCP_IPV6_END"
echo ""
echo "DNSサーバー:"
echo "  IPv4: 1.1.1.1, 8.8.8.8"
echo "  IPv6: 2606:4700:4700::1111, 2001:4860:4860::8888"