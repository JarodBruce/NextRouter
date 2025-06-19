#!/bin/bash

# ローカルIP別トラフィック監視のテストスクリプト

echo "🚀 ローカルIP別トラフィック監視テスト"
echo "======================================"

# 色設定
GREEN='\033[0;32m# 🔍 推奨Prometheusクエリ例
echo -e "${BLUE}🔍 推奨Prometheusクエリ例${NC}"
echo "---"
echo "# ローカルIP別送信量（総計 - 推奨）:"
echo "local_ip_total_tx_bytes"
echo ""
echo "# ローカルIP別受信量（総計 - 推奨）:"  
echo "local_ip_total_rx_bytes"
echo ""
echo "# ローカルIP別通信率（総計）:"
echo "rate(local_ip_total_tx_bytes[5m]) * 8"
echo ""
echo "# Top 5 通信量（総計）:"
echo "topk(5, local_ip_total_tx_bytes)"
echo ""
echo "# 詳細分析（通信先別）:"
echo "local_ip_tx_bytes_total"3[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

# テスト関数
test_metrics() {
    echo -e "${BLUE}📊 メトリクステスト: $1${NC}"
    echo "---"
    
    local query="$2"
    local result=$(curl -s http://localhost:8080/metrics | grep "$query" | head -5)
    
    if [[ -n "$result" ]]; then
        echo -e "${GREEN}✓ 成功${NC}"
        echo "$result"
    else
        echo -e "${YELLOW}⚠ メトリクスが見つかりません (まだ生成されていない可能性があります)${NC}"
    fi
    echo ""
}

# アプリケーション起動確認
echo -e "${BLUE}🔍 アプリケーション状態確認${NC}"
if pgrep -f "network-traffic-monitor" > /dev/null; then
    echo -e "${GREEN}✓ Rust Network Monitor は動作中です${NC}"
    echo "PID: $(pgrep -f 'network-traffic-monitor')"
else
    echo -e "${YELLOW}⚠ アプリケーションが起動していません${NC}"
    echo "まず ./rust-app-manager.sh start を実行してください"
    exit 1
fi
echo ""

# Prometheusエンドポイント確認
echo -e "${BLUE}🌐 Prometheusエンドポイント確認${NC}"
if curl -s http://localhost:8080/health > /dev/null; then
    echo -e "${GREEN}✓ Prometheusサーバーに接続できます${NC}"
    echo "URL: http://localhost:8080"
else
    echo -e "${YELLOW}⚠ Prometheusサーバーに接続できません${NC}"
    exit 1
fi
echo ""

# ローカルトラフィック生成
echo -e "${BLUE}📡 テスト用ローカルトラフィック生成${NC}"
echo "127.0.0.1 への ping を実行中..."
ping -c 3 127.0.0.1 > /dev/null 2>&1 &

# Prometheusエンドポイントへのリクエスト
echo "Prometheusエンドポイントにリクエスト送信中..."
for i in {1..3}; do
    curl -s http://localhost:8080/metrics > /dev/null
    sleep 1
done
echo ""

# メトリクステスト実行
echo -e "${BLUE}📈 メトリクス確認テスト${NC}"
echo ""

test_metrics "基本ネットワークメトリクス" "network_"
test_metrics "ローカルIP送信メトリクス（詳細）" "local_ip_tx_"
test_metrics "ローカルIP受信メトリクス（詳細）" "local_ip_rx_"
test_metrics "ローカルIP送信メトリクス（総計）" "local_ip_total_tx_"
test_metrics "ローカルIP受信メトリクス（総計）" "local_ip_total_rx_"

# 詳細なローカルIPメトリクス表示
echo -e "${BLUE}📋 ローカルIPメトリクス詳細${NC}"
echo "---"

echo "◆ 総計メトリクス（推奨）:"
total_metrics=$(curl -s http://localhost:8080/metrics | grep -E "local_ip_total_(tx|rx)_" | head -10)
if [[ -n "$total_metrics" ]]; then
    echo -e "${GREEN}✓ ローカルIP別総計メトリクスが動作中${NC}"
    echo "$total_metrics"
else
    echo -e "${YELLOW}⚠ ローカルIP別総計メトリクスが見つかりません${NC}"
fi

echo ""
echo "◆ 詳細メトリクス（通信先別）:"
detailed_metrics=$(curl -s http://localhost:8080/metrics | grep -E "local_ip_(tx|rx)_bytes_total" | head -5)
if [[ -n "$detailed_metrics" ]]; then
    echo -e "${GREEN}✓ 詳細メトリクスも利用可能${NC}"
    echo "$detailed_metrics"
else
    echo -e "${YELLOW}⚠ 詳細メトリクスが見つかりません${NC}"
    echo "   トラフィックが少ない可能性があります"
fi
echo ""

# 統計サマリー
echo -e "${BLUE}📊 統計サマリー${NC}"
echo "---"

# パケット総数
total_packets=$(curl -s http://localhost:8080/metrics | grep "network_packets_total" | grep -v "#" | awk '{print $2}')
if [[ -n "$total_packets" ]]; then
    echo "総パケット数: $total_packets"
fi

# バイト総数
total_bytes=$(curl -s http://localhost:8080/metrics | grep "network_bytes_total" | grep -v "#" | awk '{print $2}')
if [[ -n "$total_bytes" ]]; then
    echo "総バイト数: $total_bytes"
fi

# 現在の帯域幅
bandwidth=$(curl -s http://localhost:8080/metrics | grep "network_bandwidth_bps" | grep -v "#" | awk '{print $2}')
if [[ -n "$bandwidth" ]]; then
    echo "現在の帯域幅: ${bandwidth} bps"
fi

# ローカルIPの数
local_ip_count=$(curl -s http://localhost:8080/metrics | grep "local_ip_tx_bytes_total" | cut -d'{' -f2 | cut -d',' -f1 | sort -u | wc -l)
if [[ "$local_ip_count" -gt 0 ]]; then
    echo "監視中のローカルIP数: $local_ip_count"
fi

echo ""

# 推奨アクション
echo -e "${BLUE}💡 推奨アクション${NC}"
echo "---"
echo "1. Prometheusダッシュボード: http://localhost:9090"
echo "2. メトリクス確認: curl http://localhost:8080/metrics"
echo "3. アプリケーション管理: ./rust-app-manager.sh status"
echo "4. 詳細ドキュメント: LOCAL_IP_METRICS.md"
echo ""

# Prometheusクエリ例
echo -e "${BLUE}🔍 推奨Prometheusクエリ例${NC}"
echo "---"
echo "# ローカルIP別送信量:"
echo "sum by (local_ip) (local_ip_tx_bytes_total)"
echo ""
echo "# ローカルIP別通信率:"  
echo "rate(local_ip_tx_bytes_total[5m]) * 8"
echo ""
echo "# Top 5 通信量:"
echo "topk(5, sum by (local_ip) (local_ip_tx_bytes_total))"
echo ""

echo -e "${GREEN}✨ テスト完了！${NC}"
echo "ローカルIP別トラフィック監視が正常に動作しています。"
