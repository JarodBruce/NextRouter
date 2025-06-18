use anyhow::{Context, Result};
use log::{error, info, warn};
use pnet::datalink::{self, NetworkInterface};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::ipv6::Ipv6Packet;
use pnet::packet::Packet;
use std::net::IpAddr;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::sync::Arc;
use prometheus::{Counter, Gauge, Histogram, Registry, TextEncoder};
use tokio::time;
use std::collections::HashMap;

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
}

impl PacketCapture {
    /// 新しいPacketCaptureインスタンスを作成
    pub fn new(interface_name: &str, packet_sender: mpsc::Sender<PacketInfo>) -> Result<Self> {
        let interface = find_interface(interface_name)
            .context(format!("Failed to find interface: {}", interface_name))?;

        let metrics = Arc::new(std::sync::Mutex::new(NetworkMetrics::new()));
        let traffic_stats = Arc::new(std::sync::Mutex::new(TrafficStats::new(Duration::from_secs(10))));

        Ok(Self {
            interface,
            packet_sender,
            metrics,
            traffic_stats,
        })
    }

    /// メトリクスへの参照を取得
    pub fn get_metrics(&self) -> Arc<std::sync::Mutex<NetworkMetrics>> {
        self.metrics.clone()
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
                        // メトリクスを更新
                        if let Ok(mut metrics) = self.metrics.lock() {
                            metrics.record_packet(&packet_info);
                        }

                        // トラフィック統計を更新
                        if let Ok(mut stats) = self.traffic_stats.lock() {
                            stats.add_bytes(packet_info.size);
                            
                            // 帯域幅をメトリクスに記録
                            let bandwidth = stats.get_bandwidth_bps();
                            if let Ok(metrics) = self.metrics.lock() {
                                metrics.update_bandwidth(bandwidth);
                            }
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

    /// 生のパケットデータを解析してPacketInfoに変換
    fn parse_packet(&self, packet: &[u8]) -> Option<PacketInfo> {
        let timestamp = chrono::Utc::now();
        let size = packet.len() as u64;

        // Ethernetヘッダーの解析
        if let Some(ethernet) = EthernetPacket::new(packet) {
            match ethernet.get_ethertype() {
                EtherTypes::Ipv4 => {
                    if let Some(ipv4) = Ipv4Packet::new(ethernet.payload()) {
                        return self.parse_ipv4_packet(&ipv4, size, timestamp);
                    }
                }
                EtherTypes::Ipv6 => {
                    if let Some(ipv6) = Ipv6Packet::new(ethernet.payload()) {
                        return self.parse_ipv6_packet(&ipv6, size, timestamp);
                    }
                }
                _ => {
                    // 他のEtherTypeは無視
                    return None;
                }
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
) -> Result<(thread::JoinHandle<()>, Arc<std::sync::Mutex<NetworkMetrics>>)> {
    let capture = PacketCapture::new(interface_name, packet_sender)?;
    let metrics = capture.get_metrics();
    let interface_name = interface_name.to_string();

    let handle = thread::spawn(move || {
        info!("Starting background packet capture for interface: {}", interface_name);
        
        if let Err(e) = capture.start_capture() {
            error!("Packet capture failed for interface {}: {}", interface_name, e);
        }
        
        info!("Packet capture stopped for interface: {}", interface_name);
    });

    Ok((handle, metrics))
}

/// 完全なネットワークモニタリングシステムを開始する
pub async fn start_network_monitoring_system(
    interface_name: &str,
    _metrics_port: u16,
    _prometheus_url: Option<&str>,
    prometheus_interval_secs: u64,
) -> Result<()> {
    let (packet_sender, packet_receiver) = mpsc::channel::<PacketInfo>();
    
    // パケットキャプチャを開始
    let (_capture_handle, metrics) = start_capture_background(interface_name, packet_sender)?;
    
    info!("Network monitoring started on interface: {}", interface_name);
    
    // メトリクスのログ出力を開始
    let metrics_logger = metrics.clone();
    tokio::spawn(async move {
        if let Err(e) = log_metrics_periodically(metrics_logger, prometheus_interval_secs).await {
            error!("Metrics logging error: {}", e);
        }
    });

    // パケット処理ループ（メイン処理）
    let mut packet_count = 0u64;
    while let Ok(packet_info) = packet_receiver.recv() {
        packet_count += 1;
        
        // パケット情報をログ出力（定期的に）
        if packet_count % 1000 == 0 {
            info!("Processed {} packets. Latest: {} bytes, protocol: {}", 
                  packet_count, packet_info.size, packet_info.protocol);
        }
    }

    Ok(())
}

/// ネットワークトラフィックメトリクス構造体
#[derive(Clone)]
pub struct NetworkMetrics {
    registry: Registry,
    pub packets_total: Counter,
    pub bytes_total: Counter,
    pub packets_per_protocol: HashMap<String, Counter>,
    pub bandwidth_gauge: Gauge,
    pub packet_size_histogram: Histogram,
}

impl NetworkMetrics {
    pub fn new() -> Self {
        let registry = Registry::new();
        
        let packets_total = Counter::new("network_packets_total", "Total network packets captured").unwrap();
        let bytes_total = Counter::new("network_bytes_total", "Total network bytes captured").unwrap();
        let bandwidth_gauge = Gauge::new("network_bandwidth_bps", "Current network bandwidth in bits per second").unwrap();
        let packet_size_histogram = Histogram::with_opts(
            prometheus::HistogramOpts::new("network_packet_size_bytes", "Packet size distribution in bytes")
                .buckets(vec![64.0, 128.0, 256.0, 512.0, 1024.0, 1500.0, 2048.0, 4096.0, 8192.0])
        ).unwrap();

        // レジストリにメトリクスを登録
        registry.register(Box::new(packets_total.clone())).unwrap();
        registry.register(Box::new(bytes_total.clone())).unwrap();
        registry.register(Box::new(bandwidth_gauge.clone())).unwrap();
        registry.register(Box::new(packet_size_histogram.clone())).unwrap();

        NetworkMetrics {
            registry,
            packets_total,
            bytes_total,
            packets_per_protocol: HashMap::new(),
            bandwidth_gauge,
            packet_size_histogram,
        }
    }

    pub fn record_packet(&mut self, packet_info: &PacketInfo) {
        // 基本メトリクスを更新
        self.packets_total.inc();
        self.bytes_total.inc_by(packet_info.size as f64);
        self.packet_size_histogram.observe(packet_info.size as f64);

        // プロトコル別カウンターを更新
        let protocol = packet_info.protocol.clone();
        if !self.packets_per_protocol.contains_key(&protocol) {
            let counter = Counter::new(
                format!("network_packets_{}_total", protocol.to_lowercase()),
                format!("Total {} packets", protocol)
            ).unwrap();
            
            // レジストリに登録
            self.registry.register(Box::new(counter.clone())).unwrap();
            self.packets_per_protocol.insert(protocol.clone(), counter);
        }
        
        if let Some(counter) = self.packets_per_protocol.get(&protocol) {
            counter.inc();
        }
    }

    pub fn update_bandwidth(&self, bps: f64) {
        self.bandwidth_gauge.set(bps);
    }

    pub fn export(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode_to_string(&metric_families).unwrap()
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

/// Prometheusメトリクスをログに出力する機能（デバッグ用）
pub async fn log_metrics_periodically(
    metrics: Arc<std::sync::Mutex<NetworkMetrics>>,
    interval_secs: u64,
) -> Result<()> {
    let mut interval = time::interval(Duration::from_secs(interval_secs));

    loop {
        interval.tick().await;
        
        if let Ok(metrics) = metrics.lock() {
            let metrics_data = metrics.export();
            info!("=== Network Metrics ===");
            for line in metrics_data.lines().take(10) { // 最初の10行のみログ出力
                if !line.starts_with('#') && !line.trim().is_empty() {
                    info!("{}", line);
                }
            }
            info!("========================");
        }
    }
}


