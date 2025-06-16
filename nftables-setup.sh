#!/bin/bash

# Simple Router Setup Script
# IPv4 NAPT Only (For Experiments)
# Usage: sudo ./nftables-setup.sh [WAN_INTERFACE] [LAN_INTERFACE]
# Example: sudo ./nftables-setup.sh enxc8a362d31ba2 enp1s0

# === Interface Settings ===
# Use command line arguments if provided, otherwise use default values
WAN_INTERFACE="${1:-enxc8a362d31ba2}"     # WAN (external network)
LAN_INTERFACE="${2:-enp1s0}"              # LAN (internal network)

# === Network Settings ===
LAN_IPV4_NETWORK="10.40.0.0/24"        # LAN IPv4 network
LAN_IPV4_GATEWAY="10.40.0.1/24"        # LAN IPv4 gateway
# IPv6 is dynamically set by DHCP-PD (Prefix Delegation)
DHCP_IPV4_START="10.40.0.100"          # DHCP IPv4 start address
DHCP_IPV4_END="10.40.0.200"            # DHCP IPv4 end address

echo "=== Simple Router Setup Script (IPv4 Only) ==="
echo "Execution time: $(date)"
echo "WAN Interface: $WAN_INTERFACE"
echo "LAN Interface: $LAN_INTERFACE"
echo "LAN IPv4: $LAN_IPV4_NETWORK (Gateway: ${LAN_IPV4_GATEWAY%/*})"
echo "DNS: 1.1.1.1 (Cloudflare)"
echo ""

# === Permission Check ===
if [ "$EUID" -ne 0 ]; then
    echo "Error: This script must be run with root privileges"
    echo "Usage: sudo $0 [WAN_INTERFACE] [LAN_INTERFACE]"
    exit 1
fi

# Interface existence check
if ! ip link show "$WAN_INTERFACE" &>/dev/null; then
    echo "Error: WAN interface '$WAN_INTERFACE' not found"
    echo "Available interfaces:"
    ip link show | grep -E '^[0-9]+:' | awk -F': ' '{print "  " $2}' | sed 's/@.*//' 
    exit 1
fi

if ! ip link show "$LAN_INTERFACE" &>/dev/null; then
    echo "Error: LAN interface '$LAN_INTERFACE' not found"
    echo "Available interfaces:"
    ip link show | grep -E '^[0-9]+:' | awk -F': ' '{print "  " $2}' | sed 's/@.*//' 
    exit 1
fi

echo "Step 1: Installing required packages..."
# Install required packages
echo "  - nftables..."
if ! command -v nft &> /dev/null; then
    apt update && apt install -y nftables
fi

echo "  - dnsmasq (DHCPv4 server)..."
if ! command -v dnsmasq &> /dev/null; then
    apt update && apt install -y dnsmasq
fi

# Stop and disable unnecessary services
echo "  - Stopping unnecessary IPv6 services..."
systemctl stop wide-dhcpv6-client 2>/dev/null || true
systemctl disable wide-dhcpv6-client 2>/dev/null || true
systemctl stop radvd 2>/dev/null || true
systemctl disable radvd 2>/dev/null || true

# Remove IPv6 bridge if exists
IPV6_BRIDGE="br-ipv6"
if ip link show "$IPV6_BRIDGE" &>/dev/null; then
    echo "  - Removing existing IPv6 bridge..."
    ip link set dev "$IPV6_BRIDGE" down 2>/dev/null || true
    brctl delbr "$IPV6_BRIDGE" 2>/dev/null || true
fi

echo ""
echo "Step 2: Configuring network interfaces..."
# Stop existing services
echo "  Stopping existing services..."
systemctl stop dnsmasq 2>/dev/null || true

# Configure LAN interface with IPv4 address
echo "  Configuring LAN interface ($LAN_INTERFACE)..."
# Clear existing IP addresses
ip addr flush dev "$LAN_INTERFACE" 2>/dev/null || true
ip addr add "$LAN_IPV4_GATEWAY" dev "$LAN_INTERFACE"
ip link set "$LAN_INTERFACE" up

echo "  Configuring WAN interface ($WAN_INTERFACE)..."
ip link set "$WAN_INTERFACE" up

echo ""
echo "Step 3: Setting kernel parameters..."
# Enable IP forwarding
echo "  Enabling IPv4 forwarding..."
echo 1 > /proc/sys/net/ipv4/ip_forward

# Disable IPv6 (for experiments)
echo "  Disabling IPv6..."
echo 1 > /proc/sys/net/ipv6/conf/all/disable_ipv6
echo 1 > /proc/sys/net/ipv6/conf/default/disable_ipv6

# Clear existing rules
echo "  Clearing existing nftables rules..."
nft flush ruleset

echo ""
echo "Step 4: Configuring nftables firewall..."
# Create tables and chains
echo "  Creating basic tables and chains..."
nft add table inet filter
nft add table inet nat

# Create filter table chains
nft add chain inet filter input { type filter hook input priority 0\; policy drop\; }
nft add chain inet filter forward { type filter hook forward priority 0\; policy drop\; }
nft add chain inet filter output { type filter hook output priority 0\; policy accept\; }

# Create NAT table chains
nft add chain inet nat prerouting { type nat hook prerouting priority -100\; }
nft add chain inet nat postrouting { type nat hook postrouting priority 100\; }

echo "  Setting up basic firewall rules..."

# Allow local loopback
nft add rule inet filter input iif lo accept
nft add rule inet filter output oif lo accept

# Allow established and related connections
nft add rule inet filter input ct state established,related accept
nft add rule inet filter forward ct state established,related accept

# Allow input from LAN
nft add rule inet filter input iif "$LAN_INTERFACE" accept

# Allow forwarding from LAN to WAN (important)
nft add rule inet filter forward iif "$LAN_INTERFACE" oif "$WAN_INTERFACE" accept
# Allow responses from WAN to LAN (important)
nft add rule inet filter forward iif "$WAN_INTERFACE" oif "$LAN_INTERFACE" ct state established,related accept

# Allow SSH connections (if needed)
nft add rule inet filter input tcp dport 22 accept

# Allow DHCP (from LAN)
nft add rule inet filter input iif "$LAN_INTERFACE" udp dport 67 accept

# Allow DNS forwarding
nft add rule inet filter input iif "$LAN_INTERFACE" udp dport 53 accept
nft add rule inet filter input iif "$LAN_INTERFACE" tcp dport 53 accept

# Allow ICMP (ping)
nft add rule inet filter input icmp type echo-request accept
nft add rule inet filter forward icmp type echo-request accept
nft add rule inet filter forward icmp type echo-reply accept

# NAT configuration - masquerade LAN to WAN
nft add rule inet nat postrouting oif "$WAN_INTERFACE" ip saddr "${LAN_IPV4_NETWORK}" masquerade

# Additional LAN network settings
nft add rule inet filter input ip saddr "${LAN_IPV4_NETWORK}" accept
nft add rule inet filter forward ip saddr "${LAN_IPV4_NETWORK}" accept

echo "  Saving nftables configuration..."
# Save configuration
nft list ruleset > /etc/nftables.conf

echo ""
echo "Step 5: Configuring DNS..."
# Disable systemd-resolved DNS stub listener
echo "  Configuring systemd-resolved..."
mkdir -p /etc/systemd/resolved.conf.d
cat > /etc/systemd/resolved.conf.d/dns.conf << EOF
[Resolve]
DNSStubListener=no
EOF

# Restart systemd-resolved
systemctl restart systemd-resolved

# Update DNS configuration to use Cloudflare DNS
echo "  Updating DNS configuration..."
rm -f /etc/resolv.conf
cat > /etc/resolv.conf << EOF
nameserver 1.1.1.1
EOF

echo ""
echo "Step 6: Making kernel parameters persistent..."
# Backup existing sysctl configuration
if [ ! -f /etc/sysctl.conf.backup ]; then
    cp /etc/sysctl.conf /etc/sysctl.conf.backup
fi

# Make IP forwarding persistent
echo "  Making IPv4 forwarding settings persistent..."
# Remove existing settings to avoid duplicates, then add new ones
grep -v "net.ipv4.ip_forward\|net.ipv6.conf\|net.bridge" /etc/sysctl.conf > /tmp/sysctl.conf.new
cat >> /tmp/sysctl.conf.new << EOF

# Simple Router Configuration (IPv4 Only)
net.ipv4.ip_forward=1
# Disable IPv6 (for experiments)
net.ipv6.conf.all.disable_ipv6=1
net.ipv6.conf.default.disable_ipv6=1
EOF

mv /tmp/sysctl.conf.new /etc/sysctl.conf
sysctl -p

echo ""
echo "Step 7: Configuring DHCP/DNS server (dnsmasq)..."
# Install radvd (IPv6 Router Advertisement) if needed
if ! command -v radvd &> /dev/null; then
    echo "  Installing radvd (for IPv6 Router Advertisement, not used in IPv4 mode)..."
    apt update && apt install -y radvd
fi

# Create radvd config file if WAN_PREFIX is set
if [ -n "$WAN_PREFIX" ]; then
    BASE_PREFIX=$(echo "$WAN_PREFIX" | cut -d: -f1-4)
    echo "  Creating radvd config file..."
    cat > /etc/radvd.conf << EOF
# Router Advertisement config for IPv6 bridge
interface $IPV6_BRIDGE {
    AdvSendAdvert on;
    MinRtrAdvInterval 30;
    MaxRtrAdvInterval 600;
    
    prefix ${BASE_PREFIX}::/64 {
        AdvOnLink on;
        AdvAutonomous on;
        AdvRouterAddr on;
    };
    
    RDNSS 2001:4860:4860::8888 2001:4860:4860::8844 {
        AdvRDNSSLifetime 600;
    };
};
EOF
    # Enable and start radvd service
    systemctl enable radvd
    systemctl restart radvd
else
    echo "  radvd will be configured later (no IPv6 address on WAN)"
fi

echo ""
echo "Step 7: Configuring DHCP/DNS server (dnsmasq)..."
# Create dnsmasq configuration file (IPv4 only)
echo "  Creating dnsmasq configuration file..."
# Backup existing configuration
if [ -f /etc/dnsmasq.conf ] && [ ! -f /etc/dnsmasq.conf.backup ]; then
    cp /etc/dnsmasq.conf /etc/dnsmasq.conf.backup
fi

cat > /etc/dnsmasq.conf << EOF
# Simple Router dnsmasq Configuration (IPv4 Only)

# Basic settings
interface=$LAN_INTERFACE
bind-interfaces
domain-needed
bogus-priv

# DNS settings (Cloudflare)
server=1.1.1.1

# IPv4 DHCP settings
dhcp-range=$DHCP_IPV4_START,$DHCP_IPV4_END,255.255.255.0,24h
dhcp-option=option:router,${LAN_IPV4_GATEWAY%/*}
dhcp-option=option:dns-server,${LAN_IPV4_GATEWAY%/*}

# Log settings
log-dhcp
# Cache size
cache-size=500
EOF

echo ""
echo "Step 8: Starting services..."
# Enable nftables service
echo "  Enabling nftables service..."
systemctl enable nftables

# Enable and start dnsmasq service
echo "  Starting dnsmasq service..."
systemctl enable dnsmasq
systemctl restart dnsmasq

echo ""
echo "Step 9: Configuration completed - verification..."
DNSMASQ_STATUS=$(systemctl is-active dnsmasq)
NFTABLES_STATUS=$(systemctl is-active nftables)

echo "=== Simple Router Setup Complete (IPv4 Only) ==="
echo "Execution time: $(date)"
echo ""
echo "[Configuration Info]"
echo "  WAN Interface: $WAN_INTERFACE"
echo "  LAN Interface: $LAN_INTERFACE"
echo "  IPv4 Network: $LAN_IPV4_NETWORK"
echo "  IPv4 Gateway: ${LAN_IPV4_GATEWAY%/*}"
echo "  IPv4 DHCP Range: $DHCP_IPV4_START - $DHCP_IPV4_END"
echo "  DNS Server: 1.1.1.1 (Cloudflare)"
echo ""
echo "[Service Status]"
echo "  nftables (Firewall): $NFTABLES_STATUS"
echo "  dnsmasq (DHCP/DNS): $DNSMASQ_STATUS"
echo ""
echo "[Current Network Configuration]"
echo "  IPv4 LAN Address:"
ip -4 addr show dev "$LAN_INTERFACE" | grep "inet " | head -1 | awk '{print "    " $2}'
echo "  IPv4 WAN Address:"
WAN_IPV4_CURRENT=$(ip -4 addr show dev "$WAN_INTERFACE" | grep "inet " | head -1)
if [ -n "$WAN_IPV4_CURRENT" ]; then
    echo "$WAN_IPV4_CURRENT" | awk '{print "    " $2}'
else
    echo "    WAN IPv4 address not acquired yet"
fi
echo ""
echo "[Test Commands]"
echo "  IPv4 connectivity test: ping -c 3 8.8.8.8"
echo "  DNS functionality test: nslookup google.com"
echo "  Firewall check: sudo nft list ruleset"
echo ""
echo "[Client-side renewal methods]"
echo "  Linux: sudo dhclient [interface]"
echo "  Windows: ipconfig /release && ipconfig /renew"
echo "  macOS: sudo dhclient [interface]"
echo ""
echo "[Configuration Files]"
echo "  nftables: /etc/nftables.conf"
echo "  dnsmasq: /etc/dnsmasq.conf"
echo "  System settings: /etc/sysctl.conf"
echo ""
echo "[Complete] Simple IPv4 Router Setup Complete!"
echo "   LAN clients can connect to the Internet via IPv4 NAT"
