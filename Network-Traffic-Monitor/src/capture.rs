use anyhow::{Context, Result};
use log::{error, info, warn};
use pnet::datalink::{self, NetworkInterface};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::ipv6::Ipv6Packet;
use pnet::packet::Packet;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::sync::Arc;
use prometheus::{Counter, Registry, TextEncoder, CounterVec};
use tokio::time;
use std::collections::HashMap;
use crate::prometheus_server::start_prometheus_server;
use crate::stats::{IpStats, IpStatsMap};

/// パケット情報を格納する構造体
#[derive(Debug, Clone)]
pub struct PacketInfo {
    pub protocol: String,
    pub size: u64,
    pub src_ip: Option<IpAddr>,
    pub dst_ip: Option<IpAddr>,
    pub src_port: Option<u16>,
    pub dst_port: Option<u16>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// パケットキャプチャを管理する構造体
pub struct PacketCapture {
    interface: NetworkInterface,
    packet_sender: mpsc::Sender<PacketInfo>,
    metrics: Arc<std::sync::Mutex<NetworkMetrics>>,
    traffic_stats: Arc<std::sync::Mutex<TrafficStats>>,
    ip_stats: IpStatsMap,
}

impl PacketCapture {
    /// 新しいPacketCaptureインスタンスを作成
    pub fn new(interface_name: &str, packet_sender: mpsc::Sender<PacketInfo>) -> Result<Self> {
        let interface = find_interface(interface_name)
            .context(format!("Failed to find interface: {}", interface_name))?;

        let metrics = Arc::new(std::sync::Mutex::new(NetworkMetrics::new()));
        let traffic_stats = Arc::new(std::sync::Mutex::new(TrafficStats::new(Duration::from_secs(10))));
        let ip_stats = Arc::new(std::sync::Mutex::new(HashMap::new()));

        Ok(Self {
            interface,
            packet_sender,
            metrics,
            traffic_stats,
            ip_stats,
        })
    }

    /// メトリクスへの参照を取得
    pub fn get_metrics(&self) -> Arc<std::sync::Mutex<NetworkMetrics>> {
        self.metrics.clone()
    }

    /// IP統計情報への参照を取得
    pub fn get_ip_stats(&self) -> IpStatsMap {
        self.ip_stats.clone()
    }

    /// パケットキャプチャを開始
    pub fn start_capture(&self) -> Result<()> {
        info!("Starting packet capture on interface: {}", self.interface.name);

        // データリンクチャネルを作成
        let config = datalink::Config {
            write_buffer_size: 4096,
            read_buffer_size: 4096,
            read_timeout: Some(Duration::from_millis(100)),
            write_timeout: None,
            channel_type: datalink::ChannelType::Layer2,
            bpf_fd_attempts: 1000,
            linux_fanout: None,
            promiscuous: true,
            socket_fd: None,
        };

        let (_, mut rx) = match datalink::channel(&self.interface, config) {
            Ok(datalink::Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => return Err(anyhow::anyhow!("Unhandled channel type")),
            Err(e) => return Err(anyhow::anyhow!("Failed to create datalink channel: {}", e)),
        };

        // パケット処理ループ
        loop {
            match rx.next() {
                Ok(packet) => {
                    if let Some(packet_info) = self.parse_packet(packet) {
                        // IP統計を更新
                        self.update_ip_stats(&packet_info);

                        // メトリクスを更新
                        if let Ok(mut metrics) = self.metrics.lock() {
                            metrics.record_packet(&packet_info);
                        }

                        // トラフィック統計を更新
                        if let Ok(mut stats) = self.traffic_stats.lock() {
                            stats.add_bytes(packet_info.size);
                        }

                        // debug!("Captured packet: {:?}", packet_info);
                        
                        if let Err(e) = self.packet_sender.send(packet_info) {
                            error!("Failed to send packet info: {}", e);
                            break;
                        }
                    }
                }
                Err(e) => {
                    // warn!("Failed to receive packet: {}", e);
                    // タイムアウトエラーは無視して継続
                    if e.kind() == std::io::ErrorKind::TimedOut {
                        continue;
                    }
                    return Err(anyhow::anyhow!("Packet capture error: {}", e));
                }
            }
        }

        Ok(())
    }

    /// IPアドレスごとの統計情報を更新
    fn update_ip_stats(&self, packet_info: &PacketInfo) {
        if let Ok(mut ip_stats) = self.ip_stats.lock() {
            if let Some(src_ip) = packet_info.src_ip {
                let stats = ip_stats.entry(src_ip).or_default();
                stats.tx_bytes += packet_info.size;
                stats.tx_packets += 1;
            }

            if let Some(dst_ip) = packet_info.dst_ip {
                let stats = ip_stats.entry(dst_ip).or_default();
                stats.rx_bytes += packet_info.size;
                stats.rx_packets += 1;
            }
        } 
    }

    /// パケットを解析してPacketInfoを生成
    fn parse_packet(&self, packet: &[u8]) -> Option<PacketInfo> {
        let ethernet_packet = EthernetPacket::new(packet)?;

        let timestamp = chrono::Utc::now();
        let size = packet.len() as u64;

        // Ethernetヘッダーの解析
        match ethernet_packet.get_ethertype() {
            EtherTypes::Ipv4 => {
                if let Some(ipv4) = Ipv4Packet::new(ethernet_packet.payload()) {
                    return self.parse_ipv4_packet(&ipv4, size, timestamp);
                }
            }
            EtherTypes::Ipv6 => {
                if let Some(ipv6) = Ipv6Packet::new(ethernet_packet.payload()) {
                    return self.parse_ipv6_packet(&ipv6, size, timestamp);
                }
            }
            _ => {
                // 他のEtherTypeは無視
                return None;
            }
        }

        None
    }

    /// IPv4パケットの解析
    fn parse_ipv4_packet(
        &self,
        ipv4: &Ipv4Packet,
        size: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Option<PacketInfo> {
        let src_ip = Some(IpAddr::V4(ipv4.get_source()));
        let dst_ip = Some(IpAddr::V4(ipv4.get_destination()));

        Some(PacketInfo {
            protocol: "IPv4".to_string(),
            size,
            src_ip,
            dst_ip,
            src_port: None,
            dst_port: None,
            timestamp,
        })
    }

    /// IPv6パケットの解析
    fn parse_ipv6_packet(
        &self,
        ipv6: &Ipv6Packet,
        size: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Option<PacketInfo> {
        let src_ip = Some(IpAddr::V6(ipv6.get_source()));
        let dst_ip = Some(IpAddr::V6(ipv6.get_destination()));

        Some(PacketInfo {
            protocol: "IPv6".to_string(),
            size,
            src_ip,
            dst_ip,
            src_port: None,
            dst_port: None,
            timestamp,
        })
    }
}

/// 指定された名前のネットワークインターフェースを検索
pub fn find_interface(name: &str) -> Result<NetworkInterface> {
    let interfaces = datalink::interfaces();
    
    // 完全一致での検索
    if let Some(interface) = interfaces.iter().find(|iface| iface.name == name) {
        return Ok(interface.clone());
    }

    // 利用可能なインターフェースをログ出力
    warn!("Interface '{}' not found. Available interfaces:", name);
    for iface in &interfaces {
        warn!("  {} - {:?}", iface.name, iface.description);
    }

    Err(anyhow::anyhow!("Interface '{}' not found", name))
}

/// 利用可能なネットワークインターフェースの一覧を取得
pub fn list_interfaces() -> Vec<NetworkInterface> {
    datalink::interfaces()
}

/// バックグラウンドでパケットキャプチャを開始する
pub fn start_capture_background(
    interface_name: &str,
    packet_sender: mpsc::Sender<PacketInfo>,
) -> Result<(thread::JoinHandle<()>, Arc<std::sync::Mutex<NetworkMetrics>>, IpStatsMap)> {
    let capture = PacketCapture::new(interface_name, packet_sender)?;
    let metrics = capture.get_metrics();
    let ip_stats = capture.get_ip_stats();
    let interface_name = interface_name.to_string();

    let handle = thread::spawn(move || {
        info!("Starting background packet capture for interface: {}", interface_name);
        
        if let Err(e) = capture.start_capture() {
            error!("Packet capture failed for interface {}: {}", interface_name, e);
        }
        
        info!("Packet capture stopped for interface: {}", interface_name);
    });

    Ok((handle, metrics, ip_stats))
}

/// 完全なネットワークモニタリングシステムを開始する
pub async fn start_network_monitoring_system(
    interface_name: &str,
    metrics_port: u16,
    _prometheus_url: Option<&str>,
    _prometheus_interval_secs: u64,
) -> Result<()> {
    let (packet_sender, packet_receiver) = mpsc::channel::<PacketInfo>();
    
    // パケットキャプチャを開始
    let (_capture_handle, metrics, ip_stats) = start_capture_background(interface_name, packet_sender)?;
    
    // ネットワークメトリクスをprometheusサーバーに設定
    crate::prometheus_server::set_network_metrics(metrics.clone());
    
    // IP統計をprometheusサーバーに設定
    crate::prometheus_server::set_ip_stats(ip_stats.clone());
    
    // Prometheusサーバーを起動（8080ポートで）
    info!("Starting Prometheus metrics server on port: {}", metrics_port);
    tokio::spawn(async move {
        if let Err(e) = start_prometheus_server(metrics_port).await {
            error!("Prometheus server error: {}", e);
        }
    });
    
    info!("Network monitoring started on interface: {}", interface_name);
    
    // メトリクスのログ出力を開始（1秒間隔）
    let metrics_logger = metrics.clone();
    tokio::spawn(async move {
        if let Err(e) = log_metrics_periodically(metrics_logger, 1).await {
            error!("Metrics logging error: {}", e);
        }
    });

    // レートメトリクス更新タスクを開始（1秒間隔）
    let metrics_rate_updater = metrics.clone();
    tokio::spawn(async move {
        if let Err(e) = update_rate_metrics_periodically(metrics_rate_updater).await {
            error!("Rate metrics update error: {}", e);
        }
    });

    // IP統計レート更新タスクを開始（1秒間隔）
    tokio::spawn(async move {
        if let Err(e) = update_ip_stats_rates_periodically(ip_stats).await {
            error!("IP stats rate update error: {}", e);
        }
    });

    // パケット処理ループ（メイン処理）
    let mut _packet_count = 0u64;
    while let Ok(_packet_info) = packet_receiver.recv() {
        _packet_count += 1;
    }

    Ok(())
}

/// ネットワークトラフィックメトリクス構造体
#[derive(Clone)]
pub struct NetworkMetrics {
    registry: Registry,
    // ローカルIP別レートメトリクス（1秒間隔）
    pub local_ip_tx_bytes_rate: prometheus::GaugeVec,  // 送信バイト数レート（ローカルIP別）
    pub local_ip_rx_bytes_rate: prometheus::GaugeVec,  // 受信バイト数レート（ローカルIP別）
    pub local_ip_tx_packets_rate: prometheus::GaugeVec, // 送信パケット数レート（ローカルIP別）
    pub local_ip_rx_packets_rate: prometheus::GaugeVec, // 受信パケット数レート（ローカルIP別）
    // 合計値用メトリクス
    pub total_tx_bytes_rate: prometheus::Gauge,  // 全ローカルIPの送信バイト数レート合計
    pub total_rx_bytes_rate: prometheus::Gauge,  // 全ローカルIPの受信バイト数レート合計
    // IP別内部カウンタ（差分計算用）
    pub internal_counters_per_ip: HashMap<String, LocalIpCounters>,
    pub last_update_time: std::time::Instant,
    // ローカルネットワーク範囲定義
    local_network_ranges: Vec<(Ipv4Addr, u8)>, // (network_addr, prefix_length)
}

#[derive(Debug, Clone)]
pub struct LocalIpCounters {
    pub tx_bytes: u64,
    pub rx_bytes: u64,
    pub tx_packets: u64,
    pub rx_packets: u64,
    pub last_tx_bytes: u64,
    pub last_rx_bytes: u64,
    pub last_tx_packets: u64,
    pub last_rx_packets: u64,
}

impl LocalIpCounters {
    pub fn new() -> Self {
        Self {
            tx_bytes: 0,
            rx_bytes: 0,
            tx_packets: 0,
            rx_packets: 0,
            last_tx_bytes: 0,
            last_rx_bytes: 0,
            last_tx_packets: 0,
            last_rx_packets: 0,
        }
    }
}

impl NetworkMetrics {
    pub fn new() -> Self {
        let registry = Registry::new();
        
        // ローカルIP別レートメトリクス（1秒間隔）
        let local_ip_tx_bytes_rate = prometheus::GaugeVec::new(
            prometheus::Opts::new("local_ip_tx_bytes_rate", "Current transmission rate in bytes/sec per local IP"),
            &["local_ip"]
        ).unwrap();
        
        let local_ip_rx_bytes_rate = prometheus::GaugeVec::new(
            prometheus::Opts::new("local_ip_rx_bytes_rate", "Current reception rate in bytes/sec per local IP"),
            &["local_ip"]
        ).unwrap();
        
        let local_ip_tx_packets_rate = prometheus::GaugeVec::new(
            prometheus::Opts::new("local_ip_tx_packets_rate", "Current transmission rate in packets/sec per local IP"),
            &["local_ip"]
        ).unwrap();
        
        let local_ip_rx_packets_rate = prometheus::GaugeVec::new(
            prometheus::Opts::new("local_ip_rx_packets_rate", "Current reception rate in packets/sec per local IP"),
            &["local_ip"]
        ).unwrap();

        // 合計値用メトリクス
        let total_tx_bytes_rate = prometheus::Gauge::new(
            "total_tx_bytes_rate",
            "Total transmission rate in bytes/sec for all local IPs",
        ).unwrap();
        
        let total_rx_bytes_rate = prometheus::Gauge::new(
            "total_rx_bytes_rate",
            "Total reception rate in bytes/sec for all local IPs",
        ).unwrap();

        // レジストリにメトリクスを登録
        registry.register(Box::new(local_ip_tx_bytes_rate.clone())).unwrap();
        registry.register(Box::new(local_ip_rx_bytes_rate.clone())).unwrap();
        registry.register(Box::new(local_ip_tx_packets_rate.clone())).unwrap();
        registry.register(Box::new(local_ip_rx_packets_rate.clone())).unwrap();
        registry.register(Box::new(total_tx_bytes_rate.clone())).unwrap();
        registry.register(Box::new(total_rx_bytes_rate.clone())).unwrap();

        // デフォルトのローカルネットワーク範囲
        let local_network_ranges = vec![
            ("10.0.0.0".parse().unwrap(), 8),      // 10.0.0.0/8
            ("172.16.0.0".parse().unwrap(), 12),   // 172.16.0.0/12
            ("192.168.0.0".parse().unwrap(), 16),  // 192.168.0.0/16
            ("127.0.0.0".parse().unwrap(), 8),     // 127.0.0.0/8 (localhost)
        ];

        NetworkMetrics {
            registry,
            local_ip_tx_bytes_rate,
            local_ip_rx_bytes_rate,
            local_ip_tx_packets_rate,
            local_ip_rx_packets_rate,
            total_tx_bytes_rate,
            total_rx_bytes_rate,
            internal_counters_per_ip: HashMap::new(),
            last_update_time: std::time::Instant::now(),
            local_network_ranges,
        }
    }

    /// IPアドレスがローカルネットワーク範囲内かどうかを判定
    fn is_local_ip(&self, ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => {
                for (network, prefix) in &self.local_network_ranges {
                    if self.ip_in_network(*ipv4, *network, *prefix) {
                        return true;
                    }
                }
                false
            }
            IpAddr::V6(_) => {
                false // IPv6はローカル判定を簡略化（今回は対象外）
            }
        }
    }

    /// IPv4アドレスが指定されたネットワーク範囲内かどうかを判定
    fn ip_in_network(&self, ip: Ipv4Addr, network: Ipv4Addr, prefix_len: u8) -> bool {
        let mask = if prefix_len == 0 {
            0
        } else {
            !((1u32 << (32 - prefix_len)) - 1)
        };
        
        let ip_u32 = u32::from(ip);
        let network_u32 = u32::from(network);
        
        (ip_u32 & mask) == (network_u32 & mask)
    }

    /// IP アドレスを文字列に変換（統計用）
    fn ip_to_string(&self, ip: &IpAddr) -> String {
        match ip {
            IpAddr::V4(ipv4) => ipv4.to_string(),
            IpAddr::V6(ipv6) => {
                // IPv6の場合、プレフィックスのみを使用（プライバシー考慮）
                let segments = ipv6.segments();
                format!("{:x}:{:x}:{:x}:{:x}::", segments[0], segments[1], segments[2], segments[3])
            }
        }
    }

    pub fn record_packet(&mut self, packet_info: &PacketInfo) {
        // ローカルIP別の統計を更新（remote_ipは無視して、local_ipのみで集約）
        if let (Some(src_ip), Some(dst_ip)) = (&packet_info.src_ip, &packet_info.dst_ip) {
            let src_is_local = self.is_local_ip(src_ip);
            let dst_is_local = self.is_local_ip(dst_ip);

            if src_is_local && !dst_is_local {
                // ローカルIPから外部への送信
                let local_ip_str = self.ip_to_string(src_ip);
                
                let counters = self.internal_counters_per_ip.entry(local_ip_str).or_insert_with(LocalIpCounters::new);
                counters.tx_bytes += packet_info.size;
                counters.tx_packets += 1;
            } else if !src_is_local && dst_is_local {
                // 外部からローカルIPへの受信
                let local_ip_str = self.ip_to_string(dst_ip);
                
                let counters = self.internal_counters_per_ip.entry(local_ip_str).or_insert_with(LocalIpCounters::new);
                counters.rx_bytes += packet_info.size;
                counters.rx_packets += 1;
            }
        }
    }

    pub fn export(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode_to_string(&metric_families).unwrap()
    }

    /// 前回からの差分を計算してレートメトリクスを更新
    pub fn update_rate_metrics(&mut self) {
        let now = std::time::Instant::now();
        let elapsed_secs = now.duration_since(self.last_update_time).as_secs_f64();
        
        // 最小間隔チェック（1秒未満は無視）
        if elapsed_secs < 1.0 {
            return;
        }

        // 合計値計算用の変数
        let mut total_tx_bytes_rate = 0.0;
        let mut total_rx_bytes_rate = 0.0;

        // 各ローカルIPのレートを計算して更新
        for (local_ip, counters) in self.internal_counters_per_ip.iter_mut() {
            // 差分計算
            let tx_bytes_diff = counters.tx_bytes - counters.last_tx_bytes;
            let rx_bytes_diff = counters.rx_bytes - counters.last_rx_bytes;
            let tx_packets_diff = counters.tx_packets - counters.last_tx_packets;
            let rx_packets_diff = counters.rx_packets - counters.last_rx_packets;

            // レート（秒あたり）を計算
            let tx_bytes_rate = (tx_bytes_diff as f64) / elapsed_secs;
            let rx_bytes_rate = (rx_bytes_diff as f64) / elapsed_secs;
            let tx_packets_rate = (tx_packets_diff as f64) / elapsed_secs;
            let rx_packets_rate = (rx_packets_diff as f64) / elapsed_secs;

            // 合計値に加算
            total_tx_bytes_rate += tx_bytes_rate;
            total_rx_bytes_rate += rx_bytes_rate;

            // Gaugeに設定
            self.local_ip_tx_bytes_rate
                .with_label_values(&[local_ip])
                .set(tx_bytes_rate);
            self.local_ip_rx_bytes_rate
                .with_label_values(&[local_ip])
                .set(rx_bytes_rate);
            self.local_ip_tx_packets_rate
                .with_label_values(&[local_ip])
                .set(tx_packets_rate);
            self.local_ip_rx_packets_rate
                .with_label_values(&[local_ip])
                .set(rx_packets_rate);

            // 前回値を更新
            counters.last_tx_bytes = counters.tx_bytes;
            counters.last_rx_bytes = counters.rx_bytes;
            counters.last_tx_packets = counters.tx_packets;
            counters.last_rx_packets = counters.rx_packets;

            // ログ出力（トラフィックがある場合のみ）
            if tx_bytes_diff > 0 || rx_bytes_diff > 0 {
                info!("Local IP {} - TX: {:.1} bytes/s, RX: {:.1} bytes/s", 
                      local_ip, tx_bytes_rate, rx_bytes_rate);
            }
        }

        // 合計値メトリクスを設定
        self.total_tx_bytes_rate.set(total_tx_bytes_rate);
        self.total_rx_bytes_rate.set(total_rx_bytes_rate);

        self.last_update_time = now;
    }

    /// ローカルIPごとの合計通信量を取得するヘルパーメソッド
    pub fn get_local_ip_summary(&self) -> HashMap<String, (f64, f64, f64, f64)> {
        let summary = HashMap::new();
        
        // CounterVecからメトリクスファミリーを取得して集計
        // ここでは簡略化し、実際の実装では prometheus クレートの API を使用して
        // 各 local_ip ラベルの値を集計する
        
        summary
    }
    
    /// ローカルネットワーク範囲を更新する
    pub fn update_local_networks(&mut self, ranges: Vec<(Ipv4Addr, u8)>) {
        self.local_network_ranges = ranges;
    }
}

/// 帯域幅計算のためのトラフィック統計
#[derive(Debug)]
pub struct TrafficStats {
    total_bytes: u64,
    last_update: std::time::Instant,
    bytes_in_window: u64,
    window_start: std::time::Instant,
    window_duration: Duration,
}

impl TrafficStats {
    pub fn new(window_duration: Duration) -> Self {
        let now = std::time::Instant::now();
        Self {
            total_bytes: 0,
            last_update: now,
            bytes_in_window: 0,
            window_start: now,
            window_duration,
        }
    }

    pub fn add_bytes(&mut self, bytes: u64) {
        self.total_bytes += bytes;
        
        let now = std::time::Instant::now();
        
        // ウィンドウをリセットする必要があるか確認
        if now.duration_since(self.window_start) >= self.window_duration {
            self.bytes_in_window = 0;
            self.window_start = now;
        }
        
        self.bytes_in_window += bytes;
        self.last_update = now;
    }

    pub fn get_bandwidth_bps(&self) -> f64 {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.window_start).as_secs_f64();
        
        if elapsed > 0.0 {
            (self.bytes_in_window as f64 * 8.0) / elapsed
        } else {
            0.0
        }
    }
}

/// Prometheusメトリクスをログに出力する機能（簡素化版）
pub async fn log_metrics_periodically(
    metrics: Arc<std::sync::Mutex<NetworkMetrics>>,
    interval_secs: u64,
) -> Result<()> {
    let mut interval = time::interval(Duration::from_secs(interval_secs));

    loop {
        interval.tick().await;
        
        if let Ok(metrics) = metrics.lock() {
            let metrics_data = metrics.export();
            
            info!("=== Network Metrics (1s interval) ===");
            
            // 基本メトリクスを表示
            let mut found_traffic = false;
            for line in metrics_data.lines() {
                if !line.starts_with('#') && !line.trim().is_empty() {
                    if line.contains("local_ip_") {
                        info!("{}", line);
                        found_traffic = true;
                    } else if line.contains("network_") && !line.contains("bandwidth") {
                        info!("{}", line);
                    }
                }
            }
            
            if !found_traffic {
                info!("No local IP traffic detected yet...");
            }
            
            info!("==========================================");
        }
    }
}

/// レートメトリクスを定期的に更新する機能
pub async fn update_rate_metrics_periodically(
    metrics: Arc<std::sync::Mutex<NetworkMetrics>>,
) -> Result<()> {
    let mut interval = time::interval(Duration::from_secs(1)); // 1秒間隔で更新

    loop {
        interval.tick().await;
        
        if let Ok(mut metrics) = metrics.lock() {
            metrics.update_rate_metrics();
        }
    }
}

/// IP統計のレートを定期的に更新する関数
pub async fn update_ip_stats_rates_periodically(
    ip_stats: IpStatsMap,
) -> Result<()> {
    let mut interval = time::interval(Duration::from_secs(1));
    let mut last_stats: HashMap<IpAddr, IpStats> = HashMap::new();
    
    loop {
        interval.tick().await;
        
        // 短時間でスナップショットを取得
        let current_snapshot = if let Ok(current_stats) = ip_stats.lock() {
            current_stats.clone()
        } else {
            warn!("Failed to acquire IP stats lock for rate calculation");
            continue;
        };
        
        // レート計算（ロック外で実行）
        let mut updated_rates: HashMap<IpAddr, (u64, u64, u64, u64)> = HashMap::new();
        for (ip, stats) in &current_snapshot {
            if let Some(last) = last_stats.get(ip) {
                let tx_bytes_rate = stats.tx_bytes.saturating_sub(last.tx_bytes);
                let rx_bytes_rate = stats.rx_bytes.saturating_sub(last.rx_bytes);
                let tx_packets_rate = stats.tx_packets.saturating_sub(last.tx_packets);
                let rx_packets_rate = stats.rx_packets.saturating_sub(last.rx_packets);
                updated_rates.insert(*ip, (tx_bytes_rate, rx_bytes_rate, tx_packets_rate, rx_packets_rate));
            }
        }
        
        // 短時間でレートを更新
        if let Ok(mut current_stats) = ip_stats.lock() {
            for (ip, (tx_bytes_rate, rx_bytes_rate, tx_packets_rate, rx_packets_rate)) in updated_rates {
                if let Some(stats) = current_stats.get_mut(&ip) {
                    stats.tx_bytes_per_sec = tx_bytes_rate;
                    stats.rx_bytes_per_sec = rx_bytes_rate;
                    stats.tx_packets_per_sec = tx_packets_rate;
                    stats.rx_packets_per_sec = rx_packets_rate;
                }
            }
        } else {
            warn!("Failed to acquire IP stats lock for rate update");
        }
        
        last_stats = current_snapshot;
    }
}


