#!/bin/bash

# Grafana Configuration Script
# PrometheusデータソースとネットワークモニタリングダッシュボードをAPI経由で設定

set -e

GRAFANA_URL="http://localhost:3000"
GRAFANA_USER="admin"
GRAFANA_PASS="admin"
PROMETHEUS_URL="http://localhost:9090"

echo "⚙️ Configuring Grafana..."

# Grafanaが起動するまで待機
wait_for_grafana() {
    echo "⏳ Waiting for Grafana to be ready..."
    for i in {1..30}; do
        if curl -s "$GRAFANA_URL/api/health" > /dev/null 2>&1; then
            echo "✅ Grafana is ready"
            return 0
        fi
        echo "   Attempt $i/30: Grafana not ready yet..."
        sleep 2
    done
    echo "❌ Grafana did not start in time"
    exit 1
}

wait_for_grafana

# Prometheusデータソースを追加
echo "📊 Adding Prometheus data source..."
cat > /tmp/datasource.json << EOF
{
  "name": "Prometheus",
  "type": "prometheus",
  "url": "$PROMETHEUS_URL",
  "access": "proxy",
  "basicAuth": false,
  "isDefault": true
}
EOF

# データソースを作成（既に存在する場合は無視）
curl -s -X POST \
  -H "Content-Type: application/json" \
  -d @/tmp/datasource.json \
  -u "$GRAFANA_USER:$GRAFANA_PASS" \
  "$GRAFANA_URL/api/datasources" > /dev/null 2>&1 || echo "ℹ️  Data source might already exist"

# ネットワークモニタリングダッシュボードを作成
echo "📈 Creating Network Monitoring Dashboard..."
cat > /tmp/dashboard.json << EOF
{
  "dashboard": {
    "title": "Network Traffic Monitor",
    "tags": ["network", "rust", "prometheus"],
    "timezone": "browser",
    "panels": [
      {
        "title": "Network Metrics Overview",
        "type": "graph",
        "targets": [
          {
            "expr": "{job=\"rust-app\", __name__!=\"memory_usage_bytes\",__name__!=\"cpu_usage_percent\",__name__!=\"scrape_samples_scraped\",__name__!=\"scrape_samples_post_metric_relabeling\",__name__!=\"scrape_duration_seconds\",__name__!=\"scrape_series_added\",__name__!=\"up\",__name__!=\"active_connections\"}",
            "refId": "A",
            "legendFormat": "{{__name__}}"
          }
        ],
        "xAxis": {
          "show": true
        },
        "yAxes": [
          {
            "show": true,
            "label": "Value"
          },
          {
            "show": true
          }
        ],
        "fill": 1,
        "lineWidth": 2,
        "points": false,
        "pointradius": 5,
        "bars": false,
        "stack": false,
        "percentage": false,
        "legend": {
          "show": true,
          "values": false,
          "min": false,
          "max": false,
          "current": false,
          "total": false,
          "avg": false
        },
        "nullPointMode": "null",
        "steppedLine": false,
        "tooltip": {
          "value_type": "individual"
        },
        "timeFrom": null,
        "timeShift": null,
        "aliasColors": {},
        "seriesOverrides": [],
        "thresholds": [],
        "gridPos": {
          "h": 9,
          "w": 12,
          "x": 0,
          "y": 0
        },
        "id": 1
      },
      {
        "title": "Packet Statistics",
        "type": "stat",
        "targets": [
          {
            "expr": "packets_received_total{job=\"rust-app\"}",
            "refId": "A",
            "legendFormat": "Packets Received"
          },
          {
            "expr": "packets_sent_total{job=\"rust-app\"}",
            "refId": "B", 
            "legendFormat": "Packets Sent"
          },
          {
            "expr": "bytes_received_total{job=\"rust-app\"}",
            "refId": "C",
            "legendFormat": "Bytes Received"
          },
          {
            "expr": "bytes_sent_total{job=\"rust-app\"}",
            "refId": "D",
            "legendFormat": "Bytes Sent"
          }
        ],
        "gridPos": {
          "h": 9,
          "w": 12,
          "x": 12,
          "y": 0
        },
        "id": 2,
        "options": {
          "values": false,
          "calcs": [
            "lastNotNull"
          ],
          "fields": "",
          "orientation": "auto",
          "textMode": "auto",
          "colorMode": "value",
          "graphMode": "area",
          "justifyMode": "auto"
        },
        "pluginVersion": "8.0.0"
      },
      {
        "title": "Traffic Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(bytes_received_total{job=\"rust-app\"}[1m])",
            "refId": "A",
            "legendFormat": "Bytes Received/sec"
          },
          {
            "expr": "rate(bytes_sent_total{job=\"rust-app\"}[1m])",
            "refId": "B",
            "legendFormat": "Bytes Sent/sec"
          }
        ],
        "gridPos": {
          "h": 9,
          "w": 24,
          "x": 0,
          "y": 9
        },
        "id": 3,
        "yAxes": [
          {
            "show": true,
            "label": "Bytes/sec"
          },
          {
            "show": true
          }
        ]
      }
    ],
    "time": {
      "from": "now-5m",
      "to": "now"
    },
    "timepicker": {
      "refresh_intervals": [
        "5s",
        "10s",
        "30s",
        "1m",
        "5m",
        "15m",
        "30m",
        "1h",
        "2h",
        "1d"
      ]
    },
    "refresh": "5s"
  },
  "overwrite": true
}
EOF

# ダッシュボードをインポート
curl -s -X POST \
  -H "Content-Type: application/json" \
  -d @/tmp/dashboard.json \
  -u "$GRAFANA_USER:$GRAFANA_PASS" \
  "$GRAFANA_URL/api/dashboards/db" > /dev/null

# 一時ファイルを削除
rm -f /tmp/datasource.json /tmp/dashboard.json

echo "✅ Grafana configuration completed!"
echo ""
echo "🌐 Access your dashboard:"
echo "   URL: $GRAFANA_URL"
echo "   Username: $GRAFANA_USER"
echo "   Password: $GRAFANA_PASS"
echo ""
echo "📊 The Network Traffic Monitor dashboard has been created with your specified metrics filter"
echo "🔄 Dashboard will refresh every 5 seconds"
