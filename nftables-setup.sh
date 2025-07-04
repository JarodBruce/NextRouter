#!/bin/bash

# === Interface Settings ===
# Use command line arguments if provided, otherwise use default values
WAN_INTERFACE="${1}"     # WAN (external network)
LAN_INTERFACE="${2}"     # LAN (internal network)

# === Network Settings ===
LAN_IPV4_NETWORK="10.40.0.0/24"        # LAN IPv4 network
LAN_IPV4_GATEWAY="10.40.0.1/24"        # LAN IPv4 gateway
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
if ! command -v nft &> /dev/null; then
    apt update && apt install -y nftables dnsmasq
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
# Load nftables rules from file
echo "  Loading nftables rules from /etc/nftables/rules.nft..."
# Substitute variables in the template file and load it
envsubst < /etc/nftables/rules.nft | nft -f -

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
# Create dnsmasq configuration file (IPv4 only)
echo "  Creating dnsmasq configuration file..."
# Backup existing configuration
if [ -f /etc/dnsmasq.conf ] && [ ! -f /etc/dnsmasq.conf.backup ]; then
    cp /etc/dnsmasq.conf /etc/dnsmasq.conf.backup
fi

# Substitute variables in the template file and create the final config
envsubst < /etc/dnsmasq/dnsmasq.conf > /etc/dnsmasq.conf

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
