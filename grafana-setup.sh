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
    
    # Grafana GPGキーを追加
    sudo mkdir -p /etc/apt/keyrings/
    wget -q -O - https://apt.grafana.com/gpg.key | gpg --dearmor | sudo tee /etc/apt/keyrings/grafana.gpg > /dev/null
    
    # Grafanaリポジトリを追加
    echo "deb [signed-by=/etc/apt/keyrings/grafana.gpg] https://apt.grafana.com stable main" | sudo tee -a /etc/apt/sources.list.d/grafana.list
    
    # パッケージリストを更新してGrafanaをインストール
    sudo apt update
    sudo apt install -y grafana
    
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
