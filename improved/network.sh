#!/bin/bash

# Source utility functions
source "$(dirname "$0")/utils.sh"

# Function to install required packages for networking
install_network_packages() {
    log "Fixing broken package dependencies..."
    sudo apt --fix-broken install -y

    log "Installing required packages for networking..."
    sudo apt update && sudo apt install -y nftables isc-dhcp-server ipcalc libpcap-dev build-essential gcc

    # Verify ipcalc installation
    if ! command -v ipcalc &> /dev/null; then
        error "ipcalc installation failed. Trying to install alternatives..."
        # Fallback: install ipcalc manually or use alternative calculation
        sudo apt install -y ipcalc-ng || sudo apt install -y sipcalc || {
            error "Could not install any IP calculator. Manual calculation will be used."
        }
    fi
}

# Function to calculate and export all necessary network variables
setup_network_variables() {
    log "Calculating network information..."
    if command -v ipcalc &> /dev/null; then
        LAN_NETWORK=$(ipcalc -n "${LAN_IP}" 2>/dev/null | awk '/Network:/ {print $2}' || echo "")
        LAN_NETMASK=$(ipcalc -m "${LAN_IP}" 2>/dev/null | grep -oP 'Netmask:\s*\K\S+' || echo "")
        LAN_BROADCAST=$(ipcalc -b "${LAN_IP}" 2>/dev/null | grep -oP 'Broadcast:\s*\K\S+' || echo "")
    else
        warning "Using manual network calculation..."
        local CALC_RESULT
        CALC_RESULT=$(calculate_network_manual "${LAN_IP}")
        LAN_NETWORK=$(echo "$CALC_RESULT" | grep "NETWORK:" | cut -d':' -f2)
        LAN_NETMASK=$(echo "$CALC_RESULT" | grep "NETMASK:" | cut -d':' -f2)
        LAN_BROADCAST=$(echo "$CALC_RESULT" | grep "BROADCAST:" | cut -d':' -f2)
    fi

    if [ -n "$LAN_NETWORK" ]; then
        LAN_NETWORK_ADDR=$(echo "${LAN_NETWORK}" | cut -d'/' -f1)
        LAN_PREFIX=$(echo "${LAN_NETWORK}" | cut -d'/' -f2)
    else
        LAN_NETWORK_ADDR=$(echo "${LAN_IP}" | cut -d'/' -f1 | cut -d'.' -f1-3).0
        LAN_PREFIX=$(echo "${LAN_IP}" | cut -d'/' -f2)
    fi

    if [ -z "$LAN_NETMASK" ] || [ "$LAN_NETMASK" = "Netmask:" ]; then
        case "${LAN_PREFIX}" in
            16) LAN_NETMASK="255.255.0.0" ;;
            8) LAN_NETMASK="255.0.0.0" ;;
            *) LAN_NETMASK="255.255.255.0" ;;
        esac
    fi

    LAN_GATEWAY=$(echo "${LAN_IP}" | cut -d'/' -f1)
    if [ -z "$LAN_BROADCAST" ]; then
        local NETWORK_BASE
        NETWORK_BASE=$(echo "${LAN_NETWORK_ADDR}" | cut -d'.' -f1-3)
        case "${LAN_PREFIX}" in
            16) LAN_BROADCAST="$(echo "${LAN_NETWORK_ADDR}" | cut -d'.' -f1-2).255.255" ;;
            8) LAN_BROADCAST="$(echo "${LAN_NETWORK_ADDR}" | cut -d'.' -f1).255.255.255" ;;
            *) LAN_BROADCAST="${NETWORK_BASE}.255" ;;
        esac
    fi

    local NETWORK_BASE
    NETWORK_BASE=$(echo "${LAN_NETWORK_ADDR}" | cut -d'.' -f1-3)
    LAN_DHCP_START="${NETWORK_BASE}.100"
    LAN_DHCP_END="${NETWORK_BASE}.200"

    export LAN_NETWORK LAN_NETMASK LAN_BROADCAST LAN_NETWORK_ADDR LAN_PREFIX LAN_GATEWAY LAN_DHCP_START LAN_DHCP_END
}

# Function to configure DHCP
configure_dhcp() {
    log "Generating DHCP configuration..."
    sudo sed -e "s/LAN_NETWORK_ADDR/${LAN_NETWORK_ADDR}/g" \
        -e "s/LAN_NETMASK/${LAN_NETMASK}/g" \
        -e "s/LAN_DHCP_START/${LAN_DHCP_START}/g" \
        -e "s/LAN_DHCP_END/${LAN_DHCP_END}/g" \
        -e "s/LAN_GATEWAY/${LAN_GATEWAY}/g" \
        -e "s/LAN_BROADCAST/${LAN_BROADCAST}/g" \
        "$(dirname "$0")/../dhcpd.conf.template" > /tmp/dhcpd.conf

    info "Generated DHCP config:"
    cat /tmp/dhcpd.conf

    sudo mv /tmp/dhcpd.conf /etc/dhcp/dhcpd.conf
    sudo sh -c "echo 'INTERFACESv4=\"${LAN0}\"' > /etc/default/isc-dhcp-server"
    sudo sh -c "echo 'INTERFACESv6=\"\"' >> /etc/default/isc-dhcp-server"
}

# Function to set up routing
setup_routing() {
    log "Setting up custom routing tables..."
    
    WAN1_GW=$(ip route show default | grep "${WAN0}" | awk '{print $3}' | head -1)
    WAN2_GW=$(ip route show default | grep "${WAN1}" | awk '{print $3}' | head -1)

    grep -q "^1[[:space:]]wan1" /etc/iproute2/rt_tables || sudo sh -c "echo '1 wan1' >> /etc/iproute2/rt_tables"
    grep -q "^2[[:space:]]wan2" /etc/iproute2/rt_tables || sudo sh -c "echo '2 wan2' >> /etc/iproute2/rt_tables"

    sudo ip rule del fwmark 1 table 1 2>/dev/null || true
    sudo ip rule del fwmark 2 table 2 2>/dev/null || true
    sudo ip route flush table 1 2>/dev/null || true
    sudo ip route flush table 2 2>/dev/null || true

    sudo ip rule add fwmark 1 table 1
    sudo ip rule add fwmark 2 table 2

    if [ -n "$WAN1_GW" ]; then
        sudo ip route add default via "${WAN1_GW}" dev "${WAN0}" table 1
    fi
    sudo ip route add "${LAN_NETWORK}" dev "${LAN0}" table 1

    if [ -n "$WAN2_GW" ]; then
        sudo ip route add default via "${WAN2_GW}" dev "${WAN1}" table 2
    fi
    sudo ip route add "${LAN_NETWORK}" dev "${LAN0}" table 2
}

# Function to apply nftables rules
apply_nftables_rules() {
    log "Generating and applying nftables configuration..."
    sudo sed -e "s/INTERFACE_1/${WAN0}/g" \
        -e "s/INTERFACE_2/${WAN1}/g" \
        -e "s/INTERFACE_3/${LAN0}/g" \
        "$(dirname "$0")/../nftables.conf.template" > /etc/nftables.conf

    sudo sh -c "echo 'net.ipv4.ip_forward=1' > /etc/sysctl.d/99-ip_forward.conf"
    sudo sysctl -p /etc/sysctl.d/99-ip_forward.conf

    sudo ip addr add "${LAN_IP}" dev "${LAN0}" 2>/dev/null || true
    sudo ip link set dev "${LAN0}" up
}

# Function to create persistent routing service
create_routing_service() {
    log "Creating persistent routing service..."
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

    sudo tee /usr/local/bin/nextrouter-routing.sh > /dev/null <<EOF
#!/bin/bash
ip rule add fwmark 1 table 1 2>/dev/null || true
ip rule add fwmark 2 table 2 2>/dev/null || true
ip route add default via ${WAN1_GW} dev ${WAN0} table 1 2>/dev/null || true
ip route add ${LAN_NETWORK} dev ${LAN0} table 1 2>/dev/null || true
ip route add default via ${WAN2_GW} dev ${WAN1} table 2 2>/dev/null || true
ip route add ${LAN_NETWORK} dev ${LAN0} table 2 2>/dev/null || true
EOF
    sudo chmod +x /usr/local/bin/nextrouter-routing.sh
}

# Function to start services and show status
finalize_network_setup() {
    log "Enabling and starting network services..."
    sudo systemctl daemon-reload
    sudo systemctl enable nextrouter-routing.service
    sudo systemctl enable nftables
    sudo systemctl enable isc-dhcp-server

    sudo systemctl restart nftables
    sudo systemctl restart isc-dhcp-server
    
    success "Dual WAN setup completed!"
    info "Current routing rules:"
    sudo ip rule show
    info "Routing table 1 (WAN1):"
    sudo ip route show table 1
    info "Routing table 2 (WAN2):"
    sudo ip route show table 2
    info "nftables ruleset:"
    sudo nft list ruleset
}
