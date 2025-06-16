# Simple Linux Router Setup (IPv4 Only)

A lightweight Linux router setup script for experimental use, providing IPv4 NAT, DHCP, and DNS services using nftables and dnsmasq.

## 📋 Overview

This repository contains scripts to transform a Linux machine into a simple IPv4-only router with the following features:

- **IPv4 NAT (Network Address Translation)** - Share internet connection from WAN to LAN
- **DHCP Server** - Automatically assign IP addresses to LAN clients
- **DNS Forwarding** - Route DNS queries through Cloudflare DNS (1.1.1.1)
- **Firewall (nftables)** - Secure packet filtering and forwarding rules
- **IPv6 Disabled** - Simplified configuration for experimental use

## 🚀 Quick Start

### Prerequisites

- Linux system with root access (tested on Ubuntu/Debian)
- Two network interfaces (one for WAN, one for LAN)
- Internet connection on WAN interface

### Basic Usage

1. **Make the script executable:**
   ```bash
   sudo chmod +x nftables-setup.sh
   ```

2. **Run with default interfaces:**
   ```bash
   sudo ./nftables-setup.sh
   ```

3. **Run with custom interfaces:**
   ```bash
   sudo ./nftables-setup.sh [WAN_INTERFACE] [LAN_INTERFACE]
   ```

### Example

```bash
# Using specific network interfaces
sudo ./nftables-setup.sh eth0 eth1

# Using USB-to-Ethernet adapter and built-in ethernet
sudo ./nftables-setup.sh enxc8a362d31ba2 enp1s0
```

## 🔧 Configuration Details

### Default Network Settings

| Setting | Value |
|---------|-------|
| LAN Network | `10.40.0.0/24` |
| LAN Gateway | `10.40.0.1` |
| DHCP Range | `10.40.0.100` - `10.40.0.200` |
| DNS Server | `1.1.1.1` (Cloudflare) |
| Lease Time | 24 hours |

### What the Script Does

1. **Package Installation**
   - Installs `nftables` (firewall)
   - Installs `dnsmasq` (DHCP/DNS server)

2. **Network Interface Configuration**
   - Assigns static IP to LAN interface
   - Brings up WAN interface for DHCP

3. **Kernel Parameters**
   - Enables IPv4 forwarding
   - Disables IPv6 globally

4. **Firewall Rules (nftables)**
   - Allows traffic from LAN to WAN
   - Enables NAT/masquerading for internet sharing
   - Permits DHCP, DNS, SSH, and ICMP traffic
   - Blocks unauthorized external access

5. **DHCP/DNS Service**
   - Configures dnsmasq for IPv4 DHCP
   - Sets up DNS forwarding to Cloudflare
   - Enables automatic IP assignment for clients

## 📊 Diagnostics

### Health Check Script

Run the diagnostic script to check router status:

```bash
sudo chmod +x router-diagnosis-simple.sh
sudo ./router-diagnosis-simple.sh [WAN_INTERFACE] [LAN_INTERFACE]
```

The diagnostic script checks:
- Network interface status
- Service health (nftables, dnsmasq)
- IPv4 configuration
- Connectivity tests
- Firewall rules

### Manual Testing

**Test internet connectivity:**
```bash
ping -c 3 8.8.8.8
```

**Test DNS resolution:**
```bash
nslookup google.com
```

**Check firewall rules:**
```bash
sudo nft list ruleset
```

**View DHCP leases:**
```bash
sudo cat /var/lib/dhcp/dhcpd.leases
# or
sudo journalctl -u dnsmasq | grep DHCP
```

## 🛠️ Troubleshooting

### Common Issues

**1. No internet access from LAN clients**
- Check if WAN interface has internet connection
- Verify IPv4 forwarding: `cat /proc/sys/net/ipv4/ip_forward` (should be 1)
- Check NAT rules: `sudo nft list table inet nat`

**2. DHCP not working**
- Check dnsmasq status: `sudo systemctl status dnsmasq`
- View dnsmasq logs: `sudo journalctl -u dnsmasq`
- Verify interface configuration: `ip addr show`

**3. DNS resolution issues**
- Test DNS server: `dig @1.1.1.1 google.com`
- Check dnsmasq configuration: `sudo cat /etc/dnsmasq.conf`

### Service Management

**Restart services:**
```bash
sudo systemctl restart dnsmasq
sudo systemctl restart nftables
```

**View service logs:**
```bash
sudo journalctl -u dnsmasq -f
sudo journalctl -u nftables -f
```

**Check service status:**
```bash
sudo systemctl status dnsmasq
sudo systemctl status nftables
```

## 📁 Files and Configuration

### Generated Configuration Files

- **`/etc/nftables.conf`** - Firewall rules
- **`/etc/dnsmasq.conf`** - DHCP/DNS configuration  
- **`/etc/sysctl.conf`** - Kernel parameters
- **`/etc/resolv.conf`** - System DNS configuration

### Backup Files

The script automatically creates backups:
- **`/etc/sysctl.conf.backup`** - Original kernel settings
- **`/etc/dnsmasq.conf.backup`** - Original dnsmasq config

## 🔄 Client Configuration

### Automatic (DHCP)

Most devices will automatically receive network configuration via DHCP:
- IP address in range `10.40.0.100` - `10.40.0.200`
- Gateway: `10.40.0.1`  
- DNS: `10.40.0.1` (forwarded to Cloudflare)

### Manual IP Renewal

**Linux:**
```bash
sudo dhclient eth0
# or
sudo dhclient -r && sudo dhclient
```

**Windows:**
```cmd
ipconfig /release && ipconfig /renew
```

**macOS:**
```bash
sudo dhclient en0
```

## ⚠️ Important Notes

### Experimental Use Only

This configuration is designed for:
- Laboratory environments
- Testing networks
- Learning purposes
- Development setups

### Security Considerations

- **SSH access is enabled** - Change default SSH settings if needed
- **IPv6 is disabled** - May not be suitable for production networks
- **Simple firewall rules** - Consider additional security for production use

### Network Requirements

- Requires two separate network interfaces
- WAN interface should have internet connectivity
- LAN interface will be reconfigured (existing IP will be replaced)

## 🔧 Customization

### Changing Network Settings

Edit the script variables at the top:

```bash
LAN_IPV4_NETWORK="192.168.1.0/24"    # Change LAN network
LAN_IPV4_GATEWAY="192.168.1.1/24"    # Change gateway IP
DHCP_IPV4_START="192.168.1.100"      # Change DHCP start
DHCP_IPV4_END="192.168.1.200"        # Change DHCP end
```

### Adding Custom Firewall Rules

After setup, you can add custom nftables rules:

```bash
# Allow specific port
sudo nft add rule inet filter input tcp dport 8080 accept

# Save configuration
sudo nft list ruleset > /etc/nftables.conf
```

## 📚 Additional Resources

- [nftables Documentation](https://netfilter.org/projects/nftables/)
- [dnsmasq Manual](http://www.thekelleys.org.uk/dnsmasq/doc.html)
- [Linux Network Administration Guide](https://tldp.org/LDP/nag2/index.html)

## 🤝 Contributing

This is an experimental project. Feel free to submit issues, suggestions, or improvements.

## 📄 License

This project is provided as-is for educational and experimental purposes.
