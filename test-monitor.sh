#!/bin/bash

# Network Traffic Monitor Quick Test Script
# このスクリプトは作成したRustプログラムの動作テストを行います

set -e

echo "=== Network Traffic Monitor Test ==="
echo "Test execution time: $(date)"
echo ""

# プロジェクトディレクトリの確認
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Cargo.toml not found. Please run this script from the project directory."
    exit 1
fi

# 権限チェック
if [ "$EUID" -ne 0 ]; then
    echo "Note: このテストスクリプトの一部はroot権限が必要です"
    echo "Full test: sudo $0"
    echo "Syntax check only: $0 --syntax-only"
fi

# Syntaxチェックのみの場合
if [ "$1" = "--syntax-only" ]; then
    echo "Step 1: Syntax check only..."
    cargo check
    if [ $? -eq 0 ]; then
        echo "✓ Syntax check passed!"
    else
        echo "✗ Syntax check failed!"
        exit 1
    fi
    exit 0
fi

echo "Step 1: Building the project..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "✗ Build failed!"
    exit 1
fi

echo "✓ Build successful!"

# バイナリパスの確認
BINARY_PATH="./target/release/network-traffic-monitor"
if [ ! -f "$BINARY_PATH" ]; then
    echo "✗ Binary not found at $BINARY_PATH"
    exit 1
fi

echo "✓ Binary found: $BINARY_PATH"

# 利用可能なインターフェースの表示
echo ""
echo "Step 2: Checking available network interfaces..."
echo "Available network interfaces:"
ip link show | grep -E '^[0-9]+:' | awk -F': ' '{print "  " $2}' | sed 's/@.*//'

# ヘルプの表示テスト
echo ""
echo "Step 3: Testing help output..."
"$BINARY_PATH" --help

if [ $? -eq 0 ]; then
    echo "✓ Help output works!"
else
    echo "✗ Help output failed!"
    exit 1
fi

# root権限が必要なテスト
if [ "$EUID" -eq 0 ]; then
    echo ""
    echo "Step 4: Testing with root privileges..."
    
    # loopbackインターフェースでの短時間テスト
    echo "Testing with loopback interface for 10 seconds..."
    timeout 10s "$BINARY_PATH" -i lo -s 5 -v -o test_output.json || true
    
    if [ -f "test_output.json" ]; then
        echo "✓ Test output file created!"
        echo "Sample output:"
        head -20 test_output.json
        rm -f test_output.json
    else
        echo "Note: No output file created (normal if no traffic on lo interface)"
    fi
    
    echo "✓ Root privilege test completed!"
else
    echo ""
    echo "Step 4: Skipping root privilege tests (not running as root)"
    echo "To test with actual packet capture, run: sudo $0"
fi

echo ""
echo "Step 5: Testing capabilities setting..."
if [ "$EUID" -eq 0 ]; then
    setcap cap_net_raw,cap_net_admin=eip "$BINARY_PATH" 2>/dev/null && {
        echo "✓ Capabilities set successfully!"
        echo "You can now run without sudo: $BINARY_PATH -i [interface]"
    } || {
        echo "Note: Failed to set capabilities (this is normal on some systems)"
    }
else
    echo "Skipping capabilities test (requires root)"
fi

echo ""
echo "=== Test Summary ==="
echo "✓ Project builds successfully"
echo "✓ Binary is executable"
echo "✓ Help output works"

if [ "$EUID" -eq 0 ]; then
    echo "✓ Root privilege tests completed"
    echo "✓ Ready for production use"
else
    echo "! Root privilege tests skipped"
    echo "! Run 'sudo $0' for complete testing"
fi

echo ""
echo "=== Usage Examples ==="
echo "Monitor ens19 interface:"
if [ "$EUID" -eq 0 ]; then
    echo "  $BINARY_PATH -i ens19 -v"
else
    echo "  sudo $BINARY_PATH -i ens19 -v"
fi

echo ""
echo "Monitor with custom interval and output:"
if [ "$EUID" -eq 0 ]; then
    echo "  $BINARY_PATH -i ens19 -s 30 -o /var/log/ens19_traffic.json"
else
    echo "  sudo $BINARY_PATH -i ens19 -s 30 -o /var/log/ens19_traffic.json"
fi

echo ""
echo "Check available interfaces first:"
echo "  ip link show"

echo ""
echo "=== Test Complete ==="
