#!/bin/bash

# 引数を解析して --word の値を取得
# 引数を解析して各値を取得
while [[ $# -gt 0 ]]; do
    case "$1" in
        --wan0=*|-w0=*) WAN0="${1#*=}" ;;
        --wan1=*|-w1=*) WAN1="${1#*=}" ;;
        --lan0=*|-l0=*) LAN0="${1#*=}" ;;
        --local-ip=*|-lip=*) LANIP="${1#*=}" ;;
        --help|-h)
            cat <<EOF
使用法:
  -w0, --wan0=<WAN0のNIC>  : WAN0のネットワークインターフェースを指定
  -w1, --wan1=<WAN1のNIC>  : WAN1のネットワークインターフェースを指定
  -l0, --lan0=<LANのNIC>   : LANのネットワークインターフェースを指定
  -lip, --local-ip<LANのローカルIPアドレス/サブネットマスク>   : ローカルIPアドレスを指定
EOF
            exit 0
            ;;
        *)
            echo "不明なオプション: $1"
            echo "--help, -h でヘルプを表示"
            exit 1
            ;;
    esac
    shift
done

# --wordが指定されなかった場合
if [ -z "$WAN0" ] || [ -z "$WAN1" ] || [ -z "$LAN0" ]; then
    if [ -z "$WAN0" ]; then
        echo "エラー: -w0, --wan0=<NIC> の形式で引数を指定してください。"
    fi
    if [ -z "$WAN1" ]; then
        echo "エラー: -w1, --wan1=<NIC> の形式で引数を指定してください。"
    fi
    if [ -z "$LAN0" ]; then
        echo "エラー: -l, --lan=<NIC> の形式で引数を指定してください。"
    fi
    if [ -z "$LANIP" ]; then
        echo "エラー: -lip, --local-ip<LANのローカルIPアドレス/サブネットマスク/サブネットマスク>の形式で引数を指定してください。"
    fi
    echo "使用法: $0 --wan0=<WAN0のNIC> --wan1=<WAN1のNIC> --lan=<LANのNIC> --local-ip=<LANのローカルIPアドレス/サブネットマスク>"
    exit 1
fi


# 取得した値を表示
echo "WAN: $WAN0 $WAN1"
echo "LAN: $LAN0"
echo "IP/subnetmask: $LANIP"

INTERFACE_1="$WAN0"     # WAN1 (first external network)
INTERFACE_2="$WAN1"     # WAN2 (second external network)
INTERFACE_3="$LAN0"     # LAN (internal network)
LAN_IP="$LANIP"         # LAN IP address

sudo apt update && sudo apt install -y nftables isc-dhcp-server ipcalc

LAN_NETWORK=$(ipcalc -n ${LAN_IP} | awk '/Network:/ {print $2}')
echo "${LAN_NETWORK}"

LAN_NETWORK=$(ipcalc -n ${LAN_IP} | awk '/Network:/ {print $2}')
echo "${LAN_NETWORK}"

# Extract network components for DHCP configuration
LAN_NETWORK_ADDR=$(echo ${LAN_NETWORK} | cut -d'/' -f1)
LAN_PREFIX=$(echo ${LAN_NETWORK} | cut -d'/' -f2)

# More robust netmask extraction
LAN_NETMASK=$(ipcalc -m ${LAN_IP} | grep -oP 'Netmask:\s*\K\S+' || echo "255.255.255.0")
if [ -z "$LAN_NETMASK" ] || [ "$LAN_NETMASK" = "Netmask:" ]; then
    # Fallback: convert CIDR to netmask
    case "${LAN_PREFIX}" in
        24) LAN_NETMASK="255.255.255.0" ;;
        16) LAN_NETMASK="255.255.0.0" ;;
        8) LAN_NETMASK="255.0.0.0" ;;
        *) LAN_NETMASK="255.255.255.0" ;;  # Default to /24
    esac
fi

LAN_GATEWAY=$(echo ${LAN_IP} | cut -d'/' -f1)
LAN_BROADCAST=$(ipcalc -b ${LAN_IP} | grep -oP 'Broadcast:\s*\K\S+' || echo "192.168.1.255")

# Calculate DHCP range based on network
NETWORK_BASE=$(echo ${LAN_NETWORK_ADDR} | cut -d'.' -f1-3)
LAN_DHCP_START="${NETWORK_BASE}.100"
LAN_DHCP_END="${NETWORK_BASE}.200"

echo "DHCP Configuration:"
echo "  Network: ${LAN_NETWORK_ADDR}"
echo "  Netmask: ${LAN_NETMASK}"
echo "  Gateway: ${LAN_GATEWAY}"
echo "  DHCP Range: ${LAN_DHCP_START} - ${LAN_DHCP_END}"
echo "  Broadcast: ${LAN_BROADCAST}"

# Debug: Check if variables are properly set
echo "Debug - Variables:"
echo "  LAN_NETMASK: '${LAN_NETMASK}'"
echo "  LAN_NETWORK_ADDR: '${LAN_NETWORK_ADDR}'"

# Verify that LAN_NETMASK is not empty before proceeding
if [ -z "$LAN_NETMASK" ]; then
    echo "Error: LAN_NETMASK is empty. Setting default to 255.255.255.0"
    LAN_NETMASK="255.255.255.0"
fi
echo "  LAN_NETWORK_ADDR: '${LAN_NETWORK_ADDR}'"

# Generate DHCP configuration from template
echo "Generating DHCP configuration..."
sed -e "s/LAN_NETWORK_ADDR/${LAN_NETWORK_ADDR}/g" \
    -e "s/LAN_NETMASK/${LAN_NETMASK}/g" \
    -e "s/LAN_DHCP_START/${LAN_DHCP_START}/g" \
    -e "s/LAN_DHCP_END/${LAN_DHCP_END}/g" \
    -e "s/LAN_GATEWAY/${LAN_GATEWAY}/g" \
    -e "s/LAN_BROADCAST/${LAN_BROADCAST}/g" \
    ./dhcpd.conf.template > ./dhcpd.conf

# Debug: Show generated config before moving
echo "Generated DHCP config:"
cat ./dhcpd.conf

sudo mv ./dhcpd.conf /etc/dhcp/dhcpd.conf

sudo sh -c "echo 'INTERFACESv4=\"${INTERFACE_3}\"' > /etc/default/isc-dhcp-server"
sudo sh -c "echo 'INTERFACESv6=\"\"' >> /etc/default/isc-dhcp-server"

WAN1_IP=$(ip -4 addr show ${INTERFACE_1} | grep -oP '(?<=inet\s)\d+(\.\d+){3}')
WAN2_IP=$(ip -4 addr show ${INTERFACE_2} | grep -oP '(?<=inet\s)\d+(\.\d+){3}')
WAN1_GW=$(ip route show default | grep ${INTERFACE_1} | awk '{print $3}' | head -1)
WAN2_GW=$(ip route show default | grep ${INTERFACE_2} | awk '{print $3}' | head -1)

echo "WAN1 (${INTERFACE_1}) IP: $WAN1_IP, Gateway: $WAN1_GW"
echo "WAN2 (${INTERFACE_2}) IP: $WAN2_IP, Gateway: $WAN2_GW"

# Setup custom routing tables for packet marking
echo "Setting up custom routing tables..."

# Add custom routing table names to rt_tables if not exists
grep -q "^1[[:space:]]wan1" /etc/iproute2/rt_tables || sudo sh -c "echo '1 wan1' >> /etc/iproute2/rt_tables"
grep -q "^2[[:space:]]wan2" /etc/iproute2/rt_tables || sudo sh -c "echo '2 wan2' >> /etc/iproute2/rt_tables"

# Remove existing routing rules for marks 1 and 2
sudo ip rule del fwmark 1 table wan1 2>/dev/null || true
sudo ip rule del fwmark 2 table wan2 2>/dev/null || true
sudo ip rule del fwmark 1 table 1 2>/dev/null || true
sudo ip rule del fwmark 2 table 2 2>/dev/null || true
sudo ip rule del fwmark 1 table 10 2>/dev/null || true
sudo ip rule del fwmark 2 table 20 2>/dev/null || true

# Clear custom routing tables
sudo ip route flush table wan1 2>/dev/null || true
sudo ip route flush table wan2 2>/dev/null || true
sudo ip route flush table 1 2>/dev/null || true
sudo ip route flush table 2 2>/dev/null || true
sudo ip route flush table 10 2>/dev/null || true
sudo ip route flush table 20 2>/dev/null || true

# Add routing rules for packet marks (use numeric table IDs)
sudo ip rule add fwmark 1 table 1
sudo ip rule add fwmark 2 table 2

# Setup routing tables for WAN1 (mark 1, table 1)
sudo ip route add default via ${WAN1_GW} dev ${INTERFACE_1} table 1
sudo ip route add ${LAN_NETWORK} dev ${INTERFACE_3} table 1
# Add WAN1 network route to table
WAN1_NETWORK=$(ip route show dev ${INTERFACE_1} | grep -E '\/[0-9]+' | head -1 | awk '{print $1}')
if [ -n "$WAN1_NETWORK" ]; then
    sudo ip route add ${WAN1_NETWORK} dev ${INTERFACE_1} table 1
fi

# Setup routing tables for WAN2 (mark 2, table 2)
sudo ip route add default via ${WAN2_GW} dev ${INTERFACE_2} table 2
sudo ip route add ${LAN_NETWORK} dev ${INTERFACE_3} table 2
# Add WAN2 network route to table
WAN2_NETWORK=$(ip route show dev ${INTERFACE_2} | grep -E '\/[0-9]+' | head -1 | awk '{print $1}')
if [ -n "$WAN2_NETWORK" ]; then
    sudo ip route add ${WAN2_NETWORK} dev ${INTERFACE_2} table 2
fi

# Apply nftables rules
echo "Generating nftables configuration..."
sed -e "s/INTERFACE_1/${INTERFACE_1}/g" \
    -e "s/INTERFACE_2/${INTERFACE_2}/g" \
    -e "s/INTERFACE_3/${INTERFACE_3}/g" \
    ./nftables.conf.template > ./nftables.conf

sudo mv ./nftables.conf /etc/nftables.conf
rm -rf ./nftables.conf

sudo sh -c "echo 'net.ipv4.ip_forward=1' > /etc/sysctl.d/99-ip_forward.conf"
sudo sysctl -p /etc/sysctl.d/99-ip_forward.conf

# Configure LAN interface
sudo ip addr add ${LAN_IP} dev ${INTERFACE_3} 2>/dev/null || true
sudo ip link set dev ${INTERFACE_3} up

# Create systemd service for persistent routing rules
echo "Creating persistent routing service..."
sudo tee /etc/systemd/system/nextrouter-routing.service > /dev/null <<EOF
[Unit]
Description=NextRouter Custom Routing Rules
After=network.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/nextrouter-routing.sh
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
EOF

# Create routing script
sudo tee /usr/local/bin/nextrouter-routing.sh > /dev/null <<EOF
#!/bin/bash
# NextRouter routing rules

# Add routing rules for packet marks (using numeric table IDs)
ip rule add fwmark 1 table 1 2>/dev/null || true
ip rule add fwmark 2 table 2 2>/dev/null || true

# Setup routing tables
ip route add default via ${WAN1_GW} dev ${INTERFACE_1} table 1 2>/dev/null || true
ip route add ${LAN_NETWORK} dev ${INTERFACE_3} table 1 2>/dev/null || true
ip route add default via ${WAN2_GW} dev ${INTERFACE_2} table 2 2>/dev/null || true
ip route add ${LAN_NETWORK} dev ${INTERFACE_3} table 2 2>/dev/null || true

# Add WAN network routes if they exist
WAN1_NETWORK=\$(ip route show dev ${INTERFACE_1} | grep -E '\/[0-9]+' | head -1 | awk '{print \$1}')
if [ -n "\$WAN1_NETWORK" ]; then
    ip route add \${WAN1_NETWORK} dev ${INTERFACE_1} table 1 2>/dev/null || true
fi

WAN2_NETWORK=\$(ip route show dev ${INTERFACE_2} | grep -E '\/[0-9]+' | head -1 | awk '{print \$1}')
if [ -n "\$WAN2_NETWORK" ]; then
    ip route add \${WAN2_NETWORK} dev ${INTERFACE_2} table 2 2>/dev/null || true
fi
EOF

sudo chmod +x /usr/local/bin/nextrouter-routing.sh
sudo systemctl enable nextrouter-routing.service

echo "Dual WAN setup completed!"
echo ""
echo "IP Address Assignment:"
echo "  192.168.1.101 -> WAN1 (${INTERFACE_1})"
echo "  192.168.1.100 -> WAN2 (${INTERFACE_2})"
echo ""
echo "Routing Tables:"
echo "  Mark 1 -> Table 1 (WAN1)"
echo "  Mark 2 -> Table 2 (WAN2)"
echo ""

sudo systemctl enable nftables
sudo systemctl enable isc-dhcp-server

sudo systemctl restart nftables
sudo systemctl restart isc-dhcp-server

echo "Current routing rules:"
sudo ip rule show
echo ""
echo "Routing table 1 (WAN1):"
sudo ip route show table 1
echo ""
echo "Routing table 2 (WAN2):"
sudo ip route show table 2
echo ""
echo "nftables ruleset:"
sudo nft list ruleset


set -e

# Add these function definitions before using them
success() {
    echo "✓ $1"
}

error() {
    echo "✗ $1" >&2
}

sudo apt install -y build-essential gcc
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
success "Rustをインストールしました"
if rustup default stable; then
    success "Rustの安定版ツールチェーンを設定しました"
else
    error "Rustツールチェーンの設定に失敗しました"
    exit 1
fi

cd ./Network-Traffic-Monitor

if cargo build --release; then
    success "ビルドが完了しました"
else
    error "ビルドに失敗しました"
    exit 1
fi

# Create systemd service for network traffic monitor
echo "Creating network traffic monitor service..."
sudo tee /etc/systemd/system/network-traffic-monitor.service > /dev/null <<EOF
[Unit]
Description=Network Traffic Monitor
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/home/ubuntu/NextRouter/Network-Traffic-Monitor
ExecStart=/home/ubuntu/NextRouter/Network-Traffic-Monitor/target/release/network-traffic-monitor --interface ${LAN0}
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

# Enable and start the service
sudo systemctl daemon-reload
sudo systemctl enable network-traffic-monitor.service
sudo systemctl start network-traffic-monitor.service

success "Network Traffic Monitorサービスを登録・開始しました"

# Check service status
if sudo systemctl is-active --quiet network-traffic-monitor.service; then
    success "Network Traffic Monitorサービスが正常に動作しています"
else
    error "Network Traffic Monitorサービスの開始に失敗しました"
    sudo systemctl status network-traffic-monitor.service
fi

