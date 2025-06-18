# 🚀 Prometheus + Rust Exporter + Grafana Monitoring Stack

このプロジェクトは、RustアプリケーションからPrometheusにメトリクスを送信し、Grafanaで可視化する完全な監視スタックです。

## 📋 構成要素

- **Prometheus** - メトリクス収集・保存システム
- **Grafana** - データ可視化ダッシュボード
- **Rust Exporter** - カスタムメトリクスを生成するRustアプリケーション
- **Network Traffic Monitor** - ネットワークトラフィック監視ツール
- **Simple Linux Router** - IPv4ルーター設定 (実験用)

## 🚀 クイックスタート

### 1. システム起動

```bash
# すべてのサービスを一括起動
./setup.sh
```

### 2. 手動での起動

```bash
cd prometheus_docker
docker-compose up --build -d
```

## 📱 アクセス先

| サービス | URL | 認証情報 |
|---------|-----|----------|
| Rust Exporter | http://localhost:8080 | なし |
| Prometheus | http://localhost:9090 | なし |
| Grafana | http://localhost:3000 | admin/admin |

## 🔧 Rust Exporterエンドポイント

### メトリクス確認
```bash
curl http://localhost:8080/metrics
```

### ヘルスチェック
```bash
curl http://localhost:8080/health
```

### 処理シミュレート
```bash
curl http://localhost:8080/simulate
```

### カスタムメトリクス送信
```bash
curl -X POST http://localhost:8080/custom \
  -H 'Content-Type: application/json' \
  -d '{"value": 42.5}'
```

## 📊 利用可能なメトリクス

- `http_requests_total` - HTTPリクエスト総数
- `http_requests_in_progress` - 現在処理中のHTTPリクエスト数
- `http_request_duration_seconds` - HTTPリクエスト処理時間
- `custom_operations_total` - カスタム操作の総数
- `system_load_gauge` - システム負荷（シミュレート）

## 🔨 開発

### Rustアプリケーションの変更

```bash
cd prometheus_rust_exporter
cargo run
```

### 設定変更

- Prometheus設定: `prometheus_docker/prometheus.yml`
- Docker Compose設定: `prometheus_docker/docker-compose.yml`

## 🛑 停止・クリーンアップ

```bash
cd prometheus_docker
docker-compose down --volumes
```

## 📈 Grafanaダッシュボード設定

1. http://localhost:3000 にアクセス
2. admin/admin でログイン
3. データソース設定:
   - URL: http://prometheus:9090
   - Type: Prometheus
4. ダッシュボードでクエリ例:
   - `rate(http_requests_total[5m])` - リクエスト率
   - `http_requests_in_progress` - 進行中リクエスト
   - `system_load_gauge` - システム負荷

## 🏗️ プロジェクト構造

```
├── prometheus_docker/          # Docker環境
│   ├── docker-compose.yml     # サービス定義
│   ├── prometheus.yml         # Prometheus設定
│   └── start-monitoring.sh    # 起動スクリプト
├── prometheus_rust_exporter/   # Rustアプリケーション
│   ├── src/main.rs            # メインアプリケーション
│   ├── Cargo.toml            # 依存関係
│   └── Dockerfile            # Dockerイメージ
├── Network-Traffic-Monitor/    # ネットワーク監視ツール
└── setup.sh                  # 一括セットアップスクリプト
```

## 🔧 依存関係

- Docker & Docker Compose
- Rust (開発時)

---

# Simple Linux Router Setup (IPv4 Only) + Network Traffic Monitor

A lightweight Linux router setup script for experimental use, providing IPv4 NAT, DHCP, and DNS services using nftables and dnsmasq, plus a Rust-based network traffic monitoring tool.

## 📋 Overview

This repository contains scripts to transform a Linux machine into a simple IPv4-only router with the following features:

- **IPv4 NAT (Network Address Translation)** - Share internet connection from WAN to LAN
- **DHCP Server** - Automatically assign IP addresses to LAN clients
- **DNS Forwarding** - Route DNS queries through Cloudflare DNS (1.1.1.1)
- **Firewall (nftables)** - Secure packet filtering and forwarding rules
- **IPv6 Disabled** - Simplified configuration for experimental use
- **Network Traffic Monitor** - Real-time packet capture and analysis tool written in Rust

## 🚀 Quick Start

### Prerequisites

- Linux system with root access (tested on Ubuntu/Debian)
- Two network interfaces (one for WAN, one for LAN)
- Internet connection on WAN interface
- Rust (for building the traffic monitor)

### Quick Test (Traffic Monitor Only)

If you just want to test the network traffic monitor:

```bash
# Navigate to the traffic monitor directory
cd rust-network-sum

# Quick syntax check (no root required)
./test-monitor.sh --syntax-only

# Full test with packet capture (requires root)
sudo ./test-monitor.sh

# Check available interfaces
ip link show

# Start monitoring (example with ens19)
sudo ./target/release/network-traffic-monitor -i ens19 -v
```

### Router Setup

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

### Traffic Monitor Setup

1. **Navigate to the project directory:**
   ```bash
   cd rust-network-sum
   ```

2. **Build and install the traffic monitor:**
   ```bash
   chmod +x setup-monitor.sh
   sudo ./setup-monitor.sh
   ```

3. **Test the installation:**
   ```bash
   chmod +x test-monitor.sh
   # Syntax check only (no root required)
   ./test-monitor.sh --syntax-only
   
   # Full test (requires root)
   sudo ./test-monitor.sh
   ```

4. **Monitor network interface:**
   ```bash
   # Check available interfaces
   ip link show
   
   # Start monitoring (replace ens19 with your interface)
   sudo ./target/release/network-traffic-monitor -i ens19 -v
   ```

5. **Monitor with custom settings:**
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

# Test available interfaces first
ip link show
```

### Testing and Validation

The project includes a comprehensive test script to validate the traffic monitor:

```bash
# Change to traffic monitor directory
cd rust-network-sum

# Quick syntax and build test (no root required)
./test-monitor.sh --syntax-only

# Full functionality test (requires root privileges)
sudo ./test-monitor.sh
```

**Test script features:**
- ✓ Syntax validation and compilation check
- ✓ Binary existence and execution verification
- ✓ Help output validation
- ✓ Network interface detection
- ✓ Short-term traffic capture test (10 seconds on loopback)
- ✓ Capabilities setting for non-root execution
- ✓ Usage examples and interface listing

**Sample test output:**
```
=== Network Traffic Monitor Test ===
✓ Syntax check passed!
✓ Build successful!
✓ Binary found: ./target/release/network-traffic-monitor
✓ Help output works!
✓ Test output file created!
✓ Root privilege test completed!
✓ Capabilities set successfully!
```

### Command Line Options
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
j
### Troubleshooting

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
   
   # Rebuild project
   cd rust-network-sum
   cargo clean
   cargo build --release
   ```

4. **Test failures:**
   ```bash
   # Run syntax-only test first
   ./test-monitor.sh --syntax-only
   
   # Check if all dependencies are installed
   sudo ./setup-monitor.sh
   
   # Try manual test
   sudo ./target/release/network-traffic-monitor --help
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
### Example Router Configuration

```bash
# Set up router with traffic monitoring
sudo ./nftables-setup.sh eth0 eth1
cd rust-network-sum
sudo ./setup-monitor.sh

# Test the installation
sudo ./test-monitor.sh

# Start monitoring internal network (LAN interface)
sudo ./target/release/network-traffic-monitor -i eth1 -s 30 -o /var/log/internal_traffic.json &

# Monitor external network (WAN interface) 
sudo ./target/release/network-traffic-monitor -i eth0 -s 30 -o /var/log/external_traffic.json &

# Monitor for 1 hour then analyze
sleep 3600
pkill network-traffic-monitor

# Analyze the collected data
jq '.protocols' /var/log/internal_traffic.json | head -10
jq '.source_ips | to_entries | sort_by(.value) | reverse | .[0:5]' /var/log/internal_traffic.json
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
