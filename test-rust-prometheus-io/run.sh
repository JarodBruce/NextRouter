#!/bin/bash

echo "🚀 Prometheus Rust Client Demo"
echo "=============================="
echo

# Prometheusサーバが起動しているかチェック
echo "📡 Prometheusサーバの接続をチェック中..."
if curl -s http://localhost:9090/api/v1/query?query=up > /dev/null 2>&1; then
    echo "✅ Prometheusサーバ (http://localhost:9090) に接続できました"
    echo
    
    echo "🔧 メインアプリケーションを実行:"
    cargo run
    echo
    
    echo "📚 その他の実行例:"
    echo "  簡単な例:    cargo run --example simple"
    echo "  詳細な例:    cargo run --example detailed"
    
else
    echo "❌ Prometheusサーバ (http://localhost:9090) に接続できません"
    echo
    echo "Prometheusサーバを起動してください:"
    echo "  1. Prometheusをダウンロード: https://prometheus.io/download/"
    echo "  2. 実行: ./prometheus --config.file=prometheus.yml"
    echo "  3. ブラウザで確認: http://localhost:9090"
    echo
    echo "または、Dockerを使用:"
    echo "  docker run -p 9090:9090 prom/prometheus"
fi
