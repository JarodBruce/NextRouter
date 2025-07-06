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

sudo apt update && sudo apt install -y nftables isc-dhcp-server ipcalc

sudo cp ./dhcpd.conf /etc/dhcp/dhcpd.conf 
echo "INTERFACESv4=\"${INTERFACE_3}\"" > /etc/default/isc-dhcp-server
echo 'INTERFACESv6=""' >> /etc/default/isc-dhcp-server

WAN1_IP=$(ip -4 addr show ${INTERFACE_1} | grep -oP '(?<=inet\s)\d+(\.\d+){3}')
WAN2_IP=$(ip -4 addr show ${INTERFACE_2} | grep -oP '(?<=inet\s)\d+(\.\d+){3}')
WAN1_GW=$(ip route show default | grep ${INTERFACE_1} | awk '{print $3}' | head -1)
WAN2_GW=$(ip route show default | grep ${INTERFACE_2} | awk '{print $3}' | head -1)

echo "WAN1 (${INTERFACE_1}) IP: $WAN1_IP, Gateway: $WAN1_GW"
echo "WAN2 (${INTERFACE_2}) IP: $WAN2_IP, Gateway: $WAN2_GW"

# Apply nftables rules
echo "Generating nftables configuration..."
sed -e "s/INTERFACE_1/${INTERFACE_1}/g" \
    -e "s/INTERFACE_2/${INTERFACE_2}/g" \
    -e "s/INTERFACE_3/${INTERFACE_3}/g" \
    ./nftables.conf.template > ./nftables.conf

sudo mv ./nftables.conf /etc/nftables.conf
rm -rf ./nftables.conf
sudo ip rule del fwmark 1 table 10 2>/dev/null || true
sudo ip rule del fwmark 2 table 20 2>/dev/null || true

sudo echo "net.ipv4.ip_forward=1" > /etc/sysctl.d/99-ip_forward.conf
sudo sysctl -p /etc/sysctl.d/99-ip_forward.conf

# Add new rules
sudo ip rule add fwmark 1 table 10
sudo ip rule add fwmark 2 table 20

# Clear routing tables
sudo ip route flush table 10
sudo ip route flush table 20
sudo ip route add default via ${WAN1_GW} dev ${INTERFACE_1} table 10
sudo ip route add ${LAN_NETWORK} dev ${INTERFACE_3} table 10
sudo ip route add default via ${WAN2_GW} dev ${INTERFACE_2} table 20
sudo ip route add ${LAN_NETWORK} dev ${INTERFACE_3} table 20

# Configure LAN interface
sudo ip addr add ${LAN_IP} dev ${INTERFACE_3}
sudo ip link set dev ${INTERFACE_3} up

echo "Dual WAN setup completed!"
echo ""
echo "IP Address Assignment:"
echo "  192.168.1.101 -> WAN1 (${INTERFACE_1})"
echo "  192.168.1.100 -> WAN2 (${INTERFACE_2})"
echo ""
echo "Current routing rules:"

sudo systemctl enable nftables
sudo systemctl enable isc-dhcp-server

sudo systemctl restart nftables
sudo systemctl restart isc-dhcp-server

sudo nft list ruleset
sudo ip rule show