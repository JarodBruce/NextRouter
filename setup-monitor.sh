#!/bin/bash

# Network Traffic Monitor Setup Script
# このスクリプトは、network-traffic-monitorのビルドと実行を簡単にします

set -e

echo "=== Network Traffic Monitor Setup ==="
echo "Execution time: $(date)"
echo ""

# 権限チェック
if [ "$EUID" -ne 0 ]; then
    echo "Warning: このプログラムの実行にはroot権限が必要です"
    echo "Usage: sudo $0 [interface]"
fi

# インターフェース指定
INTERFACE="${1:-ens19}"
echo "Monitoring interface: $INTERFACE"

# 依存パッケージのチェックとインストール
echo "Step 1: Checking dependencies..."

if ! command -v cargo &> /dev/null; then
    echo "Error: Rust/Cargo not found. Please install Rust first:"
    echo "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

if ! dpkg -l | grep -q libpcap-dev; then
    echo "Installing libpcap-dev..."
    apt update
    apt install -y libpcap-dev build-essential
else
    echo "libpcap-dev is already installed"
fi

# プロジェクトのビルド
echo ""
echo "Step 2: Building the project..."
if [ -f "Cargo.toml" ]; then
    cargo build --release
    echo "Build completed successfully!"
else
    echo "Error: Cargo.toml not found. Please run this script from the project directory."
    exit 1
fi

# インターフェースの存在確認
echo ""
echo "Step 3: Checking network interface..."
if ip link show "$INTERFACE" &>/dev/null; then
    echo "Interface '$INTERFACE' found and ready for monitoring"
else
    echo "Warning: Interface '$INTERFACE' not found"
    echo "Available interfaces:"
    ip link show | grep -E '^[0-9]+:' | awk -F': ' '{print "  " $2}' | sed 's/@.*//'
    echo ""
    echo "Please specify a valid interface:"
    echo "  sudo $0 [interface_name]"
    exit 1
fi

# 実行可能ファイルの確認
BINARY_PATH="./target/release/network-traffic-monitor"
if [ -f "$BINARY_PATH" ]; then
    echo ""
    echo "Step 4: Setting up capabilities (optional)..."
    echo "Setting capabilities to run without sudo (recommended for regular use):"
    setcap cap_net_raw,cap_net_admin=eip "$BINARY_PATH" || {
        echo "Warning: Failed to set capabilities. You'll need to run with sudo."
    }
else
    echo "Error: Binary not found at $BINARY_PATH"
    exit 1
fi

echo ""
echo "=== Setup Complete ==="
echo ""
echo "You can now run the traffic monitor with:"
echo "  # With capabilities set (if successful):"
echo "  $BINARY_PATH -i $INTERFACE"
echo ""
echo "  # With sudo (always works):"
echo "  sudo $BINARY_PATH -i $INTERFACE"
echo ""
echo "  # Background monitoring with systemd-like output:"
echo "  sudo $BINARY_PATH -i $INTERFACE -o /var/log/traffic_${INTERFACE}.json &"
echo ""
echo "  # View help for all options:"
echo "  $BINARY_PATH --help"
echo ""
echo "Example usage:"
echo "  sudo $BINARY_PATH -i $INTERFACE -s 30 -v"
echo "  (Monitor $INTERFACE with 30-second intervals and verbose output)"
echo ""

# 自動実行オプションの提案
read -p "Do you want to start monitoring now? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "Starting traffic monitor..."
    echo "Press Ctrl+C to stop monitoring"
    echo ""
    
    if [ "$EUID" -eq 0 ]; then
        exec "$BINARY_PATH" -i "$INTERFACE" -v
    else
        exec sudo "$BINARY_PATH" -i "$INTERFACE" -v
    fi
fi

echo "Setup completed successfully!"
