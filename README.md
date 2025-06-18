<<<<<<< HEAD
# Simple Linux Router Setup (IPv4 Only) + Network Traffic Monitor

A lightweight Linux router setup script for experimental use, providing IPv4 NAT, DHCP, and DNS services using nftables and dnsmasq, plus a Rust-based network traffic monitoring tool.
=======
# Simple Linux Router Setup (IPv4 Only)

A lightweight Linux router setup script for experimental use, providing IPv4 NAT, DHCP, and DNS services using nftables and dnsmasq.
>>>>>>> 7fcadab0b699fe94d2d205ca5e8fcab4680db766

## 📋 Overview

This repository contains scripts to transform a Linux machine into a simple IPv4-only router with the following features:

- **IPv4 NAT (Network Address Translation)** - Share internet connection from WAN to LAN
- **DHCP Server** - Automatically assign IP addresses to LAN clients
- **DNS Forwarding** - Route DNS queries through Cloudflare DNS (1.1.1.1)
- **Firewall (nftables)** - Secure packet filtering and forwarding rules
- **IPv6 Disabled** - Simplified configuration for experimental use
<<<<<<< HEAD
- **Network Traffic Monitor** - Real-time packet capture and analysis tool written in Rust
=======
>>>>>>> 7fcadab0b699fe94d2d205ca5e8fcab4680db766

## 🚀 Quick Start

### Prerequisites

- Linux system with root access (tested on Ubuntu/Debian)
- Two network interfaces (one for WAN, one for LAN)
- Internet connection on WAN interface
<<<<<<< HEAD
- Rust (for building the traffic monitor)

### Router Setup
=======

### Basic Usage
>>>>>>> 7fcadab0b699fe94d2d205ca5e8fcab4680db766

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

<<<<<<< HEAD
### Traffic Monitor Setup

1. **Build the traffic monitor:**
   ```bash
   chmod +x setup-monitor.sh
   sudo ./setup-monitor.sh
   ```

2. **Monitor ens19 interface:**
   ```bash
   sudo ./target/release/network-traffic-monitor -i ens19 -v
   ```

3. **Monitor with custom settings:**
   ```bash
   sudo ./target/release/network-traffic-monitor -i ens19 -s 30 -o ens19_traffic.json
   ```

## 🔧 Network Traffic Monitor

### Features

- **Real-time packet monitoring** for any network interface
- **Protocol analysis** (TCP, UDP, ICMP, ARP, IPv6, etc.)
- **IP address statistics** (top source/destination IPs)
- **Port usage statistics**
- **JSON output** for data analysis
- **Configurable reporting intervals**

### Usage Examples

```bash
# Monitor ens19 interface with verbose output every 60 seconds
sudo ./target/release/network-traffic-monitor -i ens19 -v

# Monitor with 30-second intervals and custom output file
sudo ./target/release/network-traffic-monitor -i ens19 -s 30 -o /var/log/ens19_traffic.json

# Monitor multiple interfaces (run in separate terminals)
sudo ./target/release/network-traffic-monitor -i ens18 -o wan_traffic.json &
sudo ./target/release/network-traffic-monitor -i ens19 -o lan_traffic.json &

# Monitor loopback interface for testing
sudo ./target/release/network-traffic-monitor -i lo -s 10 -v
```

### Command Line Options

```
Options:
  -i, --interface <INTERFACE>  Network interface to monitor (default: ens19)
  -o, --output <OUTPUT>        Output file for statistics (default: traffic_stats.json)
  -s, --interval <INTERVAL>    Statistics aggregation interval in seconds (default: 60)
  -v, --verbose                Enable verbose logging
  -r, --realtime               Show real-time packet count
  -h, --help                   Print help
  -V, --version                Print version
```

### Sample Output

#### Console Output
```
INFO - Starting network traffic monitor
INFO - Interface: ens19
INFO - Output file: traffic_stats.json
INFO - Statistics interval: 60 seconds
INFO - === Traffic Statistics ===
INFO - Interface: ens19 | Duration: 60.0s | Total: 15234 packets, 12.46 MB | Rate: 254 packets/s, 212.67 KB/s
INFO -   TCP: 12891 packets, 11.23 MB (187.18 KB/s)
INFO -   UDP: 2134 packets, 1.03 MB (17.54 KB/s)
INFO -   ICMP: 156 packets, 156.79 KB (2.61 KB/s)
INFO -   ARP: 53 packets, 30.87 KB (514 B/s)
INFO -   Top Source IPs:
INFO -     192.168.1.100 - 5.68 MB
INFO -     192.168.1.101 - 3.46 MB
INFO -     10.0.0.50 - 1.23 MB
INFO -   Top Destination IPs:
INFO -     8.8.8.8 - 2.35 MB
INFO -     1.1.1.1 - 1.23 MB
INFO -     192.168.1.1 - 987.65 KB
INFO -   Top Ports:
INFO -     80 - 8756 packets
INFO -     443 - 6543 packets
INFO -     53 - 2341 packets
INFO - Statistics saved to: traffic_stats.json
```

#### JSON Output
```json
{
  "start_time": "2025-06-17T10:30:00Z",
  "end_time": "2025-06-17T10:31:00Z",
  "interface": "ens19",
  "total": {
    "packet_count": 15234,
    "byte_count": 12456789,
    "packets_per_second": 254.0,
    "bytes_per_second": 207613.15
  },
  "protocols": {
    "TCP": {
      "packet_count": 12891,
      "byte_count": 11234567,
      "packets_per_second": 214.85,
      "bytes_per_second": 187242.78
    },
    "UDP": {
      "packet_count": 2134,
      "byte_count": 1034567,
      "packets_per_second": 35.57,
      "bytes_per_second": 17242.78
    }
  },
  "source_ips": {
    "192.168.1.100": 5678912,
    "192.168.1.101": 3456789
  },
  "destination_ips": {
    "8.8.8.8": 2345678,
    "1.1.1.1": 1234567
  },
  "ports": {
    "80": 8756,
    "443": 6543,
    "53": 2341
  }
}
```

## 🔄 Integration with nftables Router

The traffic monitor is particularly useful for monitoring traffic on routers configured with the included `nftables-setup.sh` script:

### Monitoring Router Interfaces

```bash
# Monitor WAN interface (external traffic)
sudo ./target/release/network-traffic-monitor -i enxc8a362d31ba2 -o wan_stats.json

# Monitor LAN interface (internal traffic)  
sudo ./target/release/network-traffic-monitor -i enp1s0 -o lan_stats.json

# Monitor both interfaces for complete traffic analysis
sudo ./target/release/network-traffic-monitor -i enxc8a362d31ba2 -o wan_stats.json &
sudo ./target/release/network-traffic-monitor -i enp1s0 -o lan_stats.json &
```

### Analyzing NAPT Traffic

The traffic monitor helps analyze Network Address Port Translation (NAPT) behavior:

- **Outbound connections**: Monitor LAN interface to see internal client activity
- **Inbound traffic**: Monitor WAN interface to see external traffic patterns
- **Port usage**: Identify which services are most active
- **Bandwidth analysis**: Track data usage per IP address

## � Data Analysis

Use standard tools to analyze the JSON output:

```bash
# View latest statistics
tail -n 1 traffic_stats.json | jq .

# Extract TCP traffic over time
jq '.protocols.TCP.byte_count' traffic_stats.json

# Find top bandwidth consumers
jq '.source_ips | to_entries | sort_by(.value) | reverse | .[0:5]' traffic_stats.json

# Calculate average packets per second
jq '.total.packets_per_second' traffic_stats.json | awk '{sum+=$1; count++} END {print sum/count}'
```

## 🛠 Troubleshooting

### Traffic Monitor Issues

1. **Permission denied errors:**
   ```bash
   # Set capabilities (preferred method)
   sudo setcap cap_net_raw,cap_net_admin=eip ./target/release/network-traffic-monitor
   
   # Or always use sudo
   sudo ./target/release/network-traffic-monitor -i ens19
   ```

2. **Interface not found:**
   ```bash
   # List available interfaces
   ip link show
   
   # Use the correct interface name
   sudo ./target/release/network-traffic-monitor -i [correct_interface_name]
   ```

3. **Build errors:**
   ```bash
   # Install required dependencies
   sudo apt update
   sudo apt install build-essential libpcap-dev pkg-config
   
   # Reinstall Rust if needed
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

### Router Setup Issues

1. **Interface not found:**
   ```bash
   # Check available interfaces
   ip link show
   
   # Use correct interface names
   sudo ./nftables-setup.sh [correct_wan_interface] [correct_lan_interface]
   ```

2. **Permission errors:**
   ```bash
   # Ensure root privileges
   sudo ./nftables-setup.sh
   ```

## 📈 Performance Considerations

- The traffic monitor has minimal performance impact when monitoring typical network loads
- For high-traffic environments (>1Gbps), consider:
  - Increasing buffer sizes in the configuration
  - Using longer reporting intervals
  - Monitoring specific protocols only
- JSON log files can grow large; implement log rotation for long-term monitoring

## 🔒 Security Notes

- Both tools require root privileges for raw socket access and system configuration
- Use only in trusted environments or for experimental purposes
- Monitor log file permissions to prevent information disclosure
- Consider implementing access controls for JSON output files

## 📝 Example Router Configuration

```bash
# Set up router with traffic monitoring
sudo ./nftables-setup.sh eth0 eth1
sudo ./setup-monitor.sh eth1

# Start monitoring internal network
sudo ./target/release/network-traffic-monitor -i eth1 -s 30 -o /var/log/internal_traffic.json &

# Monitor for 1 hour then analyze
sleep 3600
pkill network-traffic-monitor

# Analyze the collected data
jq '.protocols' /var/log/internal_traffic.json | head -10
```

=======
### Example

```bash
# Using specific network interfaces
sudo ./nftables-setup.sh eth0 eth1

# Using USB-to-Ethernet adapter and built-in ethernet
sudo ./nftables-setup.sh enxc8a362d31ba2 enp1s0
```

## 🔧 Configuration Details

>>>>>>> 7fcadab0b699fe94d2d205ca5e8fcab4680db766
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
