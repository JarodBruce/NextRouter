#!/bin/bash

# Dual WAN setup script for NextRouter
# This script sets up policy-based routing for dual WAN configuration

INTERFACE_1="${1}"     # WAN1 (first external network)
INTERFACE_2="${2}"     # WAN2 (second external network)
INTERFACE_3="eth2"     # LAN (internal network)
LAN_IP="192.168.1.1/24"  # LAN IP address

sudo apt update && sudo apt install -y nftables isc-dhcp-server ipcalc

LAN_NETWORK=$(ipcalc -n ${LAN_IP} | awk '/Network:/ {print $2}')
echo "${LAN_NETWORK}"

sudo cp ./dhcpd.conf /etc/dhcp/dhcpd.conf 
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