#!/bin/bash

# Grafana Manager Script
# Grafanaサービスの管理用スクリプト

set -e

GRAFANA_URL="http://localhost:3000"
SERVICE_NAME="grafana-server"

show_usage() {
    echo "Usage: $0 {start|stop|restart|status|logs|install|configure|dashboard}"
    echo ""
    echo "Commands:"
    echo "  start      - Start Grafana service"
    echo "  stop       - Stop Grafana service"
    echo "  restart    - Restart Grafana service"
    echo "  status     - Show Grafana service status"
    echo "  logs       - Show Grafana logs"
    echo "  install    - Install and setup Grafana"
    echo "  configure  - Configure Prometheus data source and dashboard"
    echo "  dashboard  - Open dashboard URL"
    echo ""
}

check_service() {
    if systemctl is-active --quiet $SERVICE_NAME; then
        echo "✅ Grafana is running"
        echo "🌐 URL: $GRAFANA_URL"
        return 0
    else
        echo "❌ Grafana is not running"
        return 1
    fi
}

case "${1:-}" in
    start)
        echo "🚀 Starting Grafana..."
        sudo systemctl start $SERVICE_NAME
        sleep 2
        check_service
        ;;
    stop)
        echo "🛑 Stopping Grafana..."
        sudo systemctl stop $SERVICE_NAME
        echo "✅ Grafana stopped"
        ;;
    restart)
        echo "🔄 Restarting Grafana..."
        sudo systemctl restart $SERVICE_NAME
        sleep 2
        check_service
        ;;
    status)
        echo "📊 Grafana Status:"
        check_service
        echo ""
        sudo systemctl status $SERVICE_NAME --no-pager -l
        ;;
    logs)
        echo "📜 Grafana Logs:"
        sudo journalctl -u $SERVICE_NAME -f --no-pager
        ;;
    install)
        echo "📦 Installing Grafana..."
        ./grafana-setup.sh
        ;;
    configure)
        echo "⚙️ Configuring Grafana..."
        ./grafana-configure.sh
        ;;
    dashboard)
        echo "🌐 Opening Grafana Dashboard..."
        echo "URL: $GRAFANA_URL"
        if command -v xdg-open > /dev/null; then
            xdg-open "$GRAFANA_URL"
        else
            echo "Please open $GRAFANA_URL in your browser"
        fi
        ;;
    *)
        show_usage
        exit 1
        ;;
esac
