#!/bin/bash

# Grafana Setup Script for Network Monitoring
# テスト用のGrafana環境をセットアップします

set -e

GRAFANA_PORT=3000
PROMETHEUS_URL="http://localhost:9090"

echo "🚀 Starting Grafana setup..."

# Grafanaがインストールされているかチェック
if ! command -v grafana-server &> /dev/null; then
    echo "📦 Installing Grafana..."
    
    # 既存のGrafanaリポジトリ設定をクリーンアップ
    if [ -f /etc/apt/sources.list.d/grafana.list ]; then
        echo "🧹 Cleaning up existing Grafana repository configuration..."
        sudo rm -f /etc/apt/sources.list.d/grafana.list
    fi
    
    # Grafana GPGキーを追加
    sudo mkdir -p /etc/apt/keyrings/
    wget -q -O - https://apt.grafana.com/gpg.key | gpg --dearmor | sudo tee /etc/apt/keyrings/grafana.gpg > /dev/null
    
    # Grafanaリポジトリを追加（重複を避けるために新しく作成）
    echo "deb [signed-by=/etc/apt/keyrings/grafana.gpg] https://apt.grafana.com stable main" | sudo tee /etc/apt/sources.list.d/grafana.list
    
    # パッケージリストを更新（リトライ機能付き）
    echo "🔄 Updating package lists..."
    for i in {1..3}; do
        echo "Attempt $i/3..."
        if sudo apt update; then
            echo "✅ Package lists updated successfully"
            break
        else
            if [ $i -eq 3 ]; then
                echo "❌ Failed to update package lists after 3 attempts"
                echo "💡 This might be a temporary mirror sync issue. Please try again later."
                exit 1
            fi
            echo "⏳ Waiting 10 seconds before retry..."
            sleep 10
        fi
    done
    
    # Grafanaをインストール
    echo "📦 Installing Grafana package..."
    sudo apt update && sudo apt install -y grafana
    
    echo "✅ Grafana installed successfully"
else
    echo "✅ Grafana is already installed"
fi

# Grafanaサービスを開始
echo "🔄 Starting Grafana service..."
sudo systemctl start grafana-server
sudo systemctl enable grafana-server


# サービスが起動するまで少し待つ
sleep 5

echo "🌐 Grafana is now running on http://localhost:${GRAFANA_PORT}"
echo "📝 Default credentials:"
echo "   Username: admin"
echo "   Password: admin"
echo ""
echo "🔧 Next steps:"
echo "1. Open http://localhost:${GRAFANA_PORT} in your browser"
echo "2. Login with admin/admin (you'll be prompted to change password)"
echo "3. The Prometheus data source will be configured automatically"
echo "4. Import the network monitoring dashboard"
echo ""
echo "💡 To configure Prometheus data source and dashboard, run:"
echo "   ./grafana-configure.sh"
