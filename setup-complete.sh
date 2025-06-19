#!/bin/bash

# 完全統合セットアップスクリプト
# Prometheus + Grafana + Rust Network Monitor の全体セットアップ

set -e

echo "🌟 NextRouter Network Monitoring Complete Setup"
echo "=============================================="

# スクリプトに実行権限を付与
chmod +x *.sh

echo "📊 Step 1: Setting up Prometheus..."
if ! pgrep -f prometheus > /dev/null; then
    ./prometheus.sh
    sleep 3
else
    echo "✅ Prometheus is already running"
fi

echo ""
echo "📈 Step 2: Installing and configuring Grafana..."
./grafana-setup.sh
sleep 5

echo ""
echo "⚙️ Step 3: Configuring Grafana with Prometheus data source..."
./grafana-configure.sh

echo ""
echo "🦀 Step 4: Starting Rust Network Monitor..."
echo "Available network interfaces:"
./rust-app-manager.sh interfaces

echo ""
echo "Starting monitoring on loopback interface (lo) for testing..."
./rust-app-manager.sh start lo 8080 &

# 少し待ってからダッシュボードのJSONをインポート
sleep 10

echo ""
echo "📊 Step 5: Importing detailed network dashboard..."
curl -s -X POST \
  -H "Content-Type: application/json" \
  -d @network-dashboard.json \
  -u "admin:admin" \
  "http://localhost:3000/api/dashboards/db" > /dev/null || echo "ℹ️  Dashboard import might have failed, but basic setup is complete"

echo ""
echo "🎉 Setup Complete!"
echo "=================="
echo ""
echo "🌐 Access Points:"
echo "   Prometheus: http://localhost:9090"
echo "   Grafana:    http://localhost:3000 (admin/admin)"
echo ""
echo "📊 Your specific metric query is configured:"
echo '   {job="rust-app", __name__!="memory_usage_bytes",__name__!="cpu_usage_percent",__name__!="scrape_samples_scraped",__name__!="scrape_samples_post_metric_relabeling",__name__!="scrape_duration_seconds",__name__!="scrape_series_added",__name__!="up",__name__!="active_connections"}'
echo ""
echo "🔧 Management Commands:"
echo "   ./prometheus-manager.sh status"
echo "   ./grafana-manager.sh status"
echo "   ./rust-app-manager.sh status"
echo ""
echo "💡 The dashboard will show network metrics excluding system metrics"
echo "   and will refresh every 5 seconds automatically!"
