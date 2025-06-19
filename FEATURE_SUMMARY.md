# 🌐 Rustネットワーク監視 - ローカルIP別トラフィック統計機能

## 📋 改造内容サマリー

Rustネットワークトラフィックモニターに、**ローカルIP単位での通信量統計機能**を追加しました。

### ✨ 新機能

#### 1. ローカルIP別メトリクス追加

- **`local_ip_tx_bytes_total`**: ローカルIPからの送信バイト数（ラベル: local_ip, remote_ip）
- **`local_ip_rx_bytes_total`**: ローカルIPへの受信バイト数（ラベル: local_ip, remote_ip）  
- **`local_ip_tx_packets_total`**: ローカルIPからの送信パケット数
- **`local_ip_rx_packets_total`**: ローカルIPへの受信パケット数

#### 2. 自動ローカルネットワーク判定

- RFC1918プライベートアドレス範囲の自動認識
  - `10.0.0.0/8`
  - `172.16.0.0/12` 
  - `192.168.0.0/16`
  - `127.0.0.0/8` (localhost)

#### 3. 通信方向の自動分類

- **外部→ローカル**: 受信統計として記録
- **ローカル→外部**: 送信統計として記録  
- **ローカル↔ローカル**: 両方向に記録

## 🔧 変更されたファイル

### `/src/capture.rs`
- **`NetworkMetrics`構造体の拡張**: CounterVecメトリクス追加
- **IP判定ロジック追加**: `is_local_ip()`, `ip_in_network()` 
- **通信量記録ロジック**: `record_packet()` の拡張
- **プライバシー保護**: IPv6アドレスのプレフィックス処理

### `/src/prometheus_server.rs`  
- **メトリクス統合**: アプリとネットワークメトリクスの結合
- **グローバル共有**: ネットワークメトリクスの共有機能

### 新規作成ファイル
- **`LOCAL_IP_METRICS.md`**: 詳細ドキュメント
- **`test-local-ip-metrics.sh`**: テストスクリプト

## 📊 メトリクス例

```
# 送信統計
local_ip_tx_bytes_total{local_ip="192.168.1.100",remote_ip="8.8.8.8"} 1048576
local_ip_tx_packets_total{local_ip="192.168.1.100",remote_ip="8.8.8.8"} 1024

# 受信統計  
local_ip_rx_bytes_total{local_ip="192.168.1.100",remote_ip="8.8.8.8"} 2097152
local_ip_rx_packets_total{local_ip="192.168.1.100",remote_ip="8.8.8.8"} 2048
```

## 🚀 使用方法

### 1. アプリケーション起動
```bash
./rust-app-manager.sh start [interface] [port]
```

### 2. メトリクス確認
```bash
curl http://localhost:8080/metrics | grep local_ip
```

### 3. テスト実行
```bash
./test-local-ip-metrics.sh
```

## 📈 Prometheusクエリ活用例

### 基本統計
```promql
# ローカルIP別総送信量
sum by (local_ip) (local_ip_tx_bytes_total)

# ローカルIP別総受信量  
sum by (local_ip) (local_ip_rx_bytes_total)
```

### 通信レート
```promql
# 送信レート (bps)
rate(local_ip_tx_bytes_total[5m]) * 8

# パケットレート (pps)
rate(local_ip_tx_packets_total[5m])
```

### ランキング分析
```promql
# 送信量Top 10
topk(10, sum by (local_ip) (local_ip_tx_bytes_total))

# 通信先数が多いローカルIP
topk(5, count by (local_ip) (local_ip_tx_bytes_total))
```

## 🎯 実用的な応用

### 1. 帯域制御
各ローカルIPの使用帯域幅を監視し、QoS設定に活用

### 2. セキュリティ監視  
異常な通信パターンや大量通信の検出

### 3. ネットワーク最適化
最も通信量の多いデバイスを特定し、インフラ最適化

### 4. 課金・利用統計
ユーザー/デバイス別の通信量集計

## ⚡ パフォーマンス考慮

- **メトリクス数**: 通信ペア数に比例してメトリクス数が増加
- **メモリ使用量**: 大規模環境では CounterVec のカーディナリティに注意
- **CPU負荷**: パケット毎のIP判定処理追加

## 🔒 プライバシー保護

- **IPv6**: プレフィックスのみ記録（個人識別困難）
- **ローカルのみ**: ローカルIPアドレスのみ詳細記録
- **外部IP**: 必要に応じてマスキング可能

## 🎉 完成した機能

✅ ローカルIP単位での詳細通信統計  
✅ Prometheusメトリクス自動出力  
✅ 実時間監視とレート計算  
✅ プライベートネットワーク自動判定  
✅ 送信/受信方向の自動分類  
✅ テストスクリプト付属  
✅ 包括的ドキュメント完備  

---

**実装日**: 2025年6月18日  
**バージョン**: v0.1.0  
**言語**: Rust  
**監視対象**: ネットワークトラフィック（ローカルIP別）
