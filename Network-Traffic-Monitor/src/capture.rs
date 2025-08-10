use crate::prometheus_server::start_prometheus_server;
use crate::stats::IpStatsMap;
use anyhow::{Context, Result};
use log::{error, info, warn};
use pnet::datalink::{self, NetworkInterface};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::ipv6::Ipv6Packet;
use pnet::packet::Packet;
use prometheus::Registry;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::mpsc;
use pnet::packet::tcp::TcpPacket;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::time;

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

/// TCP接続の状態を追跡するための構造体
#[derive(Debug, Clone)]
pub struct TcpConnectionState {
    pub expected_seq: u32,
    pub total_packets: u64,
    pub lost_packets: u64,
    pub last_active: std::time::Instant,
}

impl TcpConnectionState {
    pub fn new(seq_num: u32, payload_len: u32) -> Self {
        Self {
            expected_seq: seq_num.wrapping_add(payload_len),
            total_packets: 1,
            lost_packets: 0,
            last_active: std::time::Instant::now(),
        }
    }
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
    pub fn new(
        interface_name: &str,
        packet_sender: mpsc::Sender<PacketInfo>,
        local_ip: Option<IpAddr>,
        local_subnet: Option<Ipv4Addr>,
    ) -> Result<Self> {
        let interface = find_interface(interface_name)
            .context(format!("Failed to find interface: {}", interface_name))?;

        let metrics = Arc::new(std::sync::Mutex::new(NetworkMetrics::new(
            local_ip,
            local_subnet,
        )));
        let traffic_stats = Arc::new(std::sync::Mutex::new(TrafficStats::new(
            Duration::from_secs(10),
        )));
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
        info!(
            "Starting packet capture on interface: {}",
            self.interface.name
        );

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

    /// シャットダウンフラグ付きでパケットキャプチャを開始
    pub fn start_capture_with_shutdown(
        &self,
        shutdown_flag: Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<()> {
        info!(
            "Starting packet capture on interface: {}",
            self.interface.name
        );

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
            // シャットダウンフラグをチェック
            if shutdown_flag.load(std::sync::atomic::Ordering::Relaxed) {
                info!("Shutdown signal received, stopping packet capture");
                break;
            }

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
            }

            if let Some(dst_ip) = packet_info.dst_ip {
                let stats = ip_stats.entry(dst_ip).or_default();
                stats.rx_bytes += packet_info.size;
            }
        }
    }

    /// パケットを解析してPacketInfoを生成
    fn parse_packet(&self, packet: &[u8]) -> Option<PacketInfo> {
        if let Some(ethernet_packet) = EthernetPacket::new(packet) {
            let timestamp = chrono::Utc::now();
            match ethernet_packet.get_ethertype() {
                EtherTypes::Ipv4 => {
                    if let Some(ipv4_packet) = Ipv4Packet::new(ethernet_packet.payload()) {
                        if ipv4_packet.get_next_level_protocol()
                            == pnet::packet::ip::IpNextHeaderProtocols::Tcp
                        {
                            if let Some(tcp_packet) =
                                pnet::packet::tcp::TcpPacket::new(ipv4_packet.payload())
                            {
                                self.detect_packet_loss(&ipv4_packet, &tcp_packet);
                            }
                        }
                        Self::parse_ipv4_packet(timestamp, &ipv4_packet)
                    } else {
                        None
                    }
                }
                EtherTypes::Ipv6 => {
                    if let Some(ipv6_packet) = Ipv6Packet::new(ethernet_packet.payload()) {
                        Self::parse_ipv6_packet(timestamp, &ipv6_packet)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        } else {
            None
        }
    }

    /// パケットロスを検出する
    fn detect_packet_loss(&self, ipv4_packet: &Ipv4Packet, tcp_packet: &TcpPacket) {
        let src_ip = ipv4_packet.get_source();
        let dst_ip = ipv4_packet.get_destination();
        let src_port = tcp_packet.get_source();
        let dst_port = tcp_packet.get_destination();
        let seq_num = tcp_packet.get_sequence();
        let payload_len = tcp_packet.payload().len() as u32;

        if payload_len == 0 {
            return;
        }

        let connection_key = format!("{}:{}-{}:{}", src_ip, src_port, dst_ip, dst_port);

        if let Ok(mut metrics) = self.metrics.lock() {
            let state = metrics
                .tcp_connection_states
                .entry(connection_key)
                .or_insert_with(|| TcpConnectionState::new(seq_num, payload_len));

            state.last_active = std::time::Instant::now();
            state.total_packets += 1;

            if seq_num > state.expected_seq {
                let gap = seq_num.wrapping_sub(state.expected_seq);
                if gap > 0 && gap < 1_000_000 {
                    // Assume gap is number of lost packets. This is a simplification.
                    state.lost_packets += 1;
                }
            }
            state.expected_seq = seq_num.wrapping_add(payload_len);
        }
    }

    /// IPv4パケットの解析
    fn parse_ipv4_packet(
        timestamp: chrono::DateTime<chrono::Utc>,
        ipv4: &Ipv4Packet,
    ) -> Option<PacketInfo> {
        let src_ip = Some(IpAddr::V4(ipv4.get_source()));
        let dst_ip = Some(IpAddr::V4(ipv4.get_destination()));

        Some(PacketInfo {
            protocol: "IPv4".to_string(),
            size: ipv4.payload().len() as u64,
            src_ip,
            dst_ip,
            src_port: None,
            dst_port: None,
            timestamp,
        })
    }

    /// IPv6パケットの解析
    fn parse_ipv6_packet(
        timestamp: chrono::DateTime<chrono::Utc>,
        ipv6: &Ipv6Packet,
    ) -> Option<PacketInfo> {
        let src_ip = Some(IpAddr::V6(ipv6.get_source()));
        let dst_ip = Some(IpAddr::V6(ipv6.get_destination()));

        Some(PacketInfo {
            protocol: "IPv6".to_string(),
            size: ipv6.payload().len() as u64,
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

/// バックグラウンドでパケットキャプチャを開始する
pub fn start_capture_background(
    interface_name: &str,
    local_ip: Option<IpAddr>,
    local_subnet: Option<Ipv4Addr>,
) -> Result<(
    Arc<std::sync::atomic::AtomicBool>,
    Arc<std::sync::Mutex<NetworkMetrics>>,
    IpStatsMap,
    mpsc::Receiver<PacketInfo>,
)> {
    let (packet_sender, packet_receiver) = mpsc::channel::<PacketInfo>();
    let capture = PacketCapture::new(interface_name, packet_sender, local_ip, local_subnet)?;
    let metrics = capture.get_metrics();
    let ip_stats = capture.get_ip_stats();
    let interface_name = interface_name.to_string();

    // シャットダウンフラグを作成
    let shutdown_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let shutdown_flag_clone = shutdown_flag.clone();

    thread::spawn(move || {
        info!(
            "Starting background packet capture for interface: {}",
            interface_name
        );

        // タイムアウト付きのパケットキャプチャを実行
        if let Err(e) = capture.start_capture_with_shutdown(shutdown_flag_clone) {
            error!(
                "Packet capture failed for interface {}: {}",
                interface_name, e
            );
        }

        info!("Packet capture stopped for interface: {}", interface_name);
    });

    Ok((shutdown_flag, metrics, ip_stats, packet_receiver))
}

/// 完全なネットワークモニタリングシステムを開始する
pub async fn start_network_monitoring_system(
    interface_name: &str,
    local_ip: Option<IpAddr>,
    local_subnet: Option<Ipv4Addr>,
) -> Result<()> {
    // パケットキャプチャを開始
    let (capture_shutdown_flag, metrics, ip_stats, packet_receiver) =
        start_capture_background(interface_name, local_ip, local_subnet)?;

    // ネットワークメトリクスをprometheusサーバーに設定
    crate::prometheus_server::set_network_metrics(metrics.clone());

    // IP統計をprometheusサーバーに設定
    crate::prometheus_server::set_ip_stats(ip_stats.clone());

    // Prometheusサーバーを起動（指定されたポートで）
    const METRICS_PORT: u16 = 59121; // メトリクスサーバーのポート
    info!(
        "Starting Prometheus metrics server on port: {}",
        METRICS_PORT
    );
    let prometheus_handle = tokio::spawn(async move {
        if let Err(e) = start_prometheus_server(METRICS_PORT).await {
            error!("Prometheus server error: {}", e);
            error!("Failed to start Prometheus server on port {}", METRICS_PORT);
        }
    });

    info!(
        "Network monitoring started on interface: {}",
        interface_name
    );

    // メトリクスのログ出力を開始（1秒間隔）
    let metrics_logger = metrics.clone();
    let log_handle = tokio::spawn(async move {
        if let Err(e) = log_metrics_periodically(metrics_logger, 1).await {
            error!("Metrics logging error: {}", e);
        }
    });

    // レートメトリクス更新タスクを開始（1秒間隔）
    let metrics_rate_updater = metrics.clone();
    let rate_update_handle = tokio::spawn(async move {
        if let Err(e) = update_rate_metrics_periodically(metrics_rate_updater).await {
            error!("Rate metrics update error: {}", e);
        }
    });

    // IP統計レート更新タスクを開始（1秒間隔）
    let ip_stats_handle = tokio::spawn(async move {
        if let Err(e) = update_ip_stats_rates_periodically(ip_stats).await {
            error!("IP stats rate updater failed: {}", e);
        }
    });

    // パケットロス率更新タスクを開始（5秒間隔）
    let metrics_packet_loss_updater = metrics.clone();
    let packet_loss_update_handle = tokio::spawn(async move {
        if let Err(e) =
            update_packet_loss_metrics_periodically(metrics_packet_loss_updater, 1).await
        {
            error!("Packet loss metrics updater failed: {}", e);
        }
    });

    // パケット処理ループ（メイン処理）
    let mut _packet_count = 0u64;

    loop {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(10)) => {
                // 非ブロッキングでパケットを受信を試行
                match packet_receiver.try_recv() {
                    Ok(_packet_info) => {
                        _packet_count += 1;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        // パケットなし、継続
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        info!("Packet receiver disconnected, stopping monitoring");
                        break;
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Shutdown signal received in monitoring system");

                // パケットキャプチャスレッドを停止
                capture_shutdown_flag.store(true, std::sync::atomic::Ordering::Relaxed);

                // 少し待ってからループを抜ける
                tokio::time::sleep(Duration::from_millis(100)).await;
                break;
            }
        }
    }

    // 全タスクを適切に終了
    info!("Stopping all monitoring tasks...");
    prometheus_handle.abort();
    log_handle.abort();
    rate_update_handle.abort();
    ip_stats_handle.abort();
    packet_loss_update_handle.abort();

    // タスクの終了を少し待つ
    tokio::time::sleep(Duration::from_millis(200)).await;

    info!("All monitoring tasks stopped");
    Ok(())
}

/// ネットワークトラフィックメトリクス構造体
#[derive(Clone)]
pub struct NetworkMetrics {
    registry: Registry,
    // ローカルIP別レートメトリクス（1秒間隔）
    pub local_ip_tx_bytes_rate: prometheus::GaugeVec, // 送信バイト数レート（ローカルIP別）
    pub local_ip_rx_bytes_rate: prometheus::GaugeVec, // 受信バイト数レート（ローカルIP別）
    // 合計値用メトリクス
    pub total_tx_bytes_rate: prometheus::Gauge, // 全ローカルIPの送信バイト数レート合計
    pub total_rx_bytes_rate: prometheus::Gauge, // 全ローカルIPの受信バイト数レート合計
    // パケットロス率メトリクス
    pub packet_loss_percentage: prometheus::Gauge, // パケットロス率（%）
    // IP別内部カウンタ（差分計算用）
    pub internal_counters_per_ip: HashMap<String, LocalIpCounters>,
    pub last_update_time: std::time::Instant,
    // ローカルネットワーク範囲定義
    local_network_ranges: Vec<(Ipv4Addr, u8)>, // (network_addr, prefix_length)
    // TCP接続追跡
    pub tcp_connection_states: HashMap<String, TcpConnectionState>,
}

#[derive(Debug, Clone)]
pub struct LocalIpCounters {
    pub tx_bytes: u64,
    pub rx_bytes: u64,
    pub last_tx_bytes: u64,
    pub last_rx_bytes: u64,
    pub last_active: std::time::Instant,
}

impl LocalIpCounters {
    pub fn new() -> Self {
        Self {
            tx_bytes: 0,
            rx_bytes: 0,
            last_tx_bytes: 0,
            last_rx_bytes: 0,
            last_active: std::time::Instant::now(),
        }
    }
}

/// 数値（bps単位）を適切な単位（bps, Kbps, Mbps, Gbps）に変換して文字列で返す
pub fn format_bps(value: f64) -> String {
    const KBPS: f64 = 1_000.0;
    const MBPS: f64 = 1_000_000.0;
    const GBPS: f64 = 1_000_000_000.0;

    if value >= GBPS {
        format!("{:.2} Gbps", value / GBPS)
    } else if value >= MBPS {
        format!("{:.2} Mbps", value / MBPS)
    } else if value >= KBPS {
        format!("{:.2} Kbps", value / KBPS)
    } else {
        format!("{:.0} bps", value)
    }
}

impl NetworkMetrics {
    pub fn new(local_ip: Option<IpAddr>, local_subnet: Option<Ipv4Addr>) -> Self {
        let registry = Registry::new();

        // ローカルIP別レートメトリクス（1秒間隔）
        let local_ip_tx_bytes_rate = prometheus::GaugeVec::new(
            prometheus::Opts::new(
                "local_ip_tx_bytes_rate",
                "Current transmission rate in bytes/sec per local IP",
            ),
            &["local_ip"],
        )
        .unwrap();

        let local_ip_rx_bytes_rate = prometheus::GaugeVec::new(
            prometheus::Opts::new(
                "local_ip_rx_bytes_rate",
                "Current reception rate in bytes/sec per local IP",
            ),
            &["local_ip"],
        )
        .unwrap();

        // 合計値用メトリクス
        let total_tx_bytes_rate = prometheus::Gauge::new(
            "total_tx_bytes_rate",
            "Total transmission rate in bytes/sec for all local IPs",
        )
        .unwrap();

        let total_rx_bytes_rate = prometheus::Gauge::new(
            "total_rx_bytes_rate",
            "Total reception rate in bytes/sec for all local IPs",
        )
        .unwrap();

        // パケットロス率メトリクス
        let packet_loss_percentage = prometheus::Gauge::new(
            "tcp_monitor_packet_loss_missing_per_second",
            "Packet loss percentage",
        )
        .unwrap();

        // レジストリにメトリクスを登録
        registry
            .register(Box::new(local_ip_tx_bytes_rate.clone()))
            .unwrap();
        registry
            .register(Box::new(local_ip_rx_bytes_rate.clone()))
            .unwrap();
        registry
            .register(Box::new(total_tx_bytes_rate.clone()))
            .unwrap();
        registry
            .register(Box::new(total_rx_bytes_rate.clone()))
            .unwrap();
        registry
            .register(Box::new(packet_loss_percentage.clone()))
            .unwrap();

        // ローカルネットワーク範囲の構築
        let local_network_ranges = Self::build_local_network_ranges(local_ip, local_subnet);

        // 構築されたローカルネットワーク範囲を表示
        info!("Configured local network ranges:");
        for (network, prefix) in &local_network_ranges {
            let (min_ip, max_ip) = Self::calculate_ip_range(*network, *prefix);
            info!("  - {}/{} ({} - {})", network, prefix, min_ip, max_ip);
        }

        NetworkMetrics {
            registry,
            local_ip_tx_bytes_rate,
            local_ip_rx_bytes_rate,
            total_tx_bytes_rate,
            total_rx_bytes_rate,
            packet_loss_percentage,
            internal_counters_per_ip: HashMap::new(),
            last_update_time: std::time::Instant::now(),
            local_network_ranges,
            tcp_connection_states: HashMap::new(),
        }
    }    /// Record a packet in the metrics
    pub fn record_packet(&mut self, packet_info: &PacketInfo) {
        // Update packet counts and byte counts based on the packet information
        if let (Some(src_ip), Some(dst_ip)) = (packet_info.src_ip, packet_info.dst_ip) {
            // Determine if this is local traffic based on configured ranges
            let is_local_src = self.is_local_ip(src_ip);
            let is_local_dst = self.is_local_ip(dst_ip);
            
            // Update metrics based on traffic direction
            if is_local_src && !is_local_dst {
                // Outbound traffic from local IP
                if let Some(local_ip_str) = self.get_local_ip_string(src_ip) {
                    let counter = self.internal_counters_per_ip.entry(local_ip_str).or_insert_with(LocalIpCounters::new);
                    counter.tx_bytes += packet_info.size;
                    counter.last_active = std::time::Instant::now();
                }
            } else if !is_local_src && is_local_dst {
                // Inbound traffic to local IP
                if let Some(local_ip_str) = self.get_local_ip_string(dst_ip) {
                    let counter = self.internal_counters_per_ip.entry(local_ip_str).or_insert_with(LocalIpCounters::new);
                    counter.rx_bytes += packet_info.size;
                    counter.last_active = std::time::Instant::now();
                }
            }
        }
    }

    /// Check if an IP address is in the local network ranges
    fn is_local_ip(&self, ip: IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => {
                for (network, prefix) in &self.local_network_ranges {
                    let mask = !((1u32 << (32 - prefix)) - 1);
                    let network_u32 = u32::from(*network);
                    let ip_u32 = u32::from(ipv4);
                    if (ip_u32 & mask) == (network_u32 & mask) {
                        return true;
                    }
                }
                false
            }
            IpAddr::V6(_) => false, // IPv6 not supported for now
        }
    }

    /// Get string representation of local IP for metrics
    fn get_local_ip_string(&self, ip: IpAddr) -> Option<String> {
        if self.is_local_ip(ip) {
            Some(ip.to_string())
        } else {
            None
        }
    }

    /// Export metrics in Prometheus format
    pub fn export(&self) -> String {
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode_to_string(&metric_families).unwrap_or_default()
    }

    /// メトリクスを更新する
    pub fn update_rate_metrics(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let now = std::time::Instant::now();
        let elapsed_secs = now.duration_since(self.last_update_time).as_secs_f64();

        // 最小間隔チェック（1秒未満は無視）
        if elapsed_secs < 1.0 {
            return Ok(());
        }

        // 合計値計算用の変数
        let mut total_tx_bytes_rate = 0.0;
        let mut total_rx_bytes_rate = 0.0;
        let mut inactive_ips = Vec::new();
        const INACTIVITY_TIMEOUT: Duration = Duration::from_secs(300); // 5分

        // 各ローカルIPのレートを計算して更新
        for (local_ip, counters) in self.internal_counters_per_ip.iter_mut() {
            // 差分計算
            let tx_bytes_diff = counters.tx_bytes - counters.last_tx_bytes;
            let rx_bytes_diff = counters.rx_bytes - counters.last_rx_bytes;

            // レート（秒あたり）を計算
            let tx_bytes_rate = (tx_bytes_diff as f64) / elapsed_secs;
            let rx_bytes_rate = (rx_bytes_diff as f64) / elapsed_secs;

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

            // 前回値を更新
            counters.last_tx_bytes = counters.tx_bytes;
            counters.last_rx_bytes = counters.rx_bytes;

            // ローカルIPのログ出力
            info!(
                "Local IP {} - TX: {}, RX: {}",
                local_ip,
                format_bps(tx_bytes_rate * 8.0),
                format_bps(rx_bytes_rate * 8.0)
            );

            // 非アクティブなIPを検出
            if now.duration_since(counters.last_active) > INACTIVITY_TIMEOUT {
                inactive_ips.push(local_ip.clone());
            }
        }

        // 非アクティブなIPを削除
        for ip in inactive_ips {
            info!("Removing inactive IP from metrics: {}", ip);

            // メトリクスから削除
            self.local_ip_tx_bytes_rate
                .with_label_values(&[&ip])
                .set(0.0);
            self.local_ip_rx_bytes_rate
                .with_label_values(&[&ip])
                .set(0.0);

            // 内部カウンタから削除
            self.internal_counters_per_ip.remove(&ip);
        }

        // 合計値メトリクスを設定
        self.total_tx_bytes_rate.set(total_tx_bytes_rate);
        self.total_rx_bytes_rate.set(total_rx_bytes_rate);
        info!(
            "Network Summary - Total TX: {}, Total RX: {}",
            format_bps(total_tx_bytes_rate * 8.0),
            format_bps(total_rx_bytes_rate * 8.0)
        );

        self.last_update_time = now;
        Ok(())
    }

    /// Build local network ranges from IP and subnet
    fn build_local_network_ranges(
        local_ip: Option<IpAddr>, 
        local_subnet: Option<Ipv4Addr>
    ) -> Vec<(Ipv4Addr, u8)> {
        let mut ranges = Vec::new();
        
        if let (Some(IpAddr::V4(ip)), Some(subnet)) = (local_ip, local_subnet) {
            let prefix = calculate_prefix_length(subnet);
            let network = calculate_network_address(ip, subnet);
            ranges.push((network, prefix));
        }
        
        ranges
    }

    /// Calculate IP range for a given network and prefix
    fn calculate_ip_range(network: Ipv4Addr, prefix: u8) -> (Ipv4Addr, Ipv4Addr) {
        let network_u32 = u32::from(network);
        let mask = !((1u32 << (32 - prefix)) - 1);
        let min_ip = Ipv4Addr::from(network_u32 & mask);
        let max_ip = Ipv4Addr::from((network_u32 & mask) | ((1u32 << (32 - prefix)) - 1));
        (min_ip, max_ip)
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
}

/// Prometheusメトリクスをログに出力する機能（簡素化版）
pub async fn log_metrics_periodically(
    _metrics: Arc<std::sync::Mutex<NetworkMetrics>>,
    interval_secs: u64,
) -> Result<()> {
    let mut interval = time::interval(Duration::from_secs(interval_secs));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                // 現在は何もしない（簡素化版）
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Metrics logger received shutdown signal");
                break;
            }
        }
    }

    Ok(())
}

/// レートメトリクスを定期的に更新する機能
pub async fn update_rate_metrics_periodically(
    metrics: Arc<std::sync::Mutex<NetworkMetrics>>,
) -> Result<()> {
    let mut interval = time::interval(Duration::from_secs(1)); // 1秒間隔で更新

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if let Ok(mut metrics) = metrics.lock() {
                    if let Err(e) = metrics.update_rate_metrics() {
                        error!("Failed to update rate metrics: {}", e);
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Rate metrics updater received shutdown signal");
                break;
            }
        }
    }

    Ok(())
}

/// IP統計のレートを定期的に更新する関数
pub async fn update_ip_stats_rates_periodically(ip_stats: IpStatsMap) -> Result<()> {
    let mut interval = time::interval(Duration::from_secs(1));
    loop {
        interval.tick().await;
        if let Ok(mut stats) = ip_stats.lock() {
            for (_, _ip_stat) in stats.iter_mut() {
                // ここでレートを計算・更新するロジックを実装
                // 現状はIpStatsにレート用のフィールドがないため、追加が必要
            }
        }
    }
}

/// パケットロス率メトリクスを定期的に更新する
pub async fn update_packet_loss_metrics_periodically(
    metrics: Arc<std::sync::Mutex<NetworkMetrics>>,
    interval_secs: u64,
) -> Result<()> {
    let mut interval = time::interval(Duration::from_secs(interval_secs));
    loop {
        interval.tick().await;
        if let Ok(mut metrics) = metrics.lock() {
            let mut total_packets = 0;
            let mut total_lost_packets = 0;

            // 古い接続をクリーンアップ
            let now = std::time::Instant::now();
            metrics
                .tcp_connection_states
                .retain(|_, state| now.duration_since(state.last_active).as_secs() < 60);

            for state in metrics.tcp_connection_states.values() {
                total_packets += state.total_packets;
                total_lost_packets += state.lost_packets;
            }

            let loss_percentage = if total_packets > 0 {
                (total_lost_packets as f64 / total_packets as f64) * 100.0
            } else {
                0.0
            };

            metrics.packet_loss_percentage.set(loss_percentage);
        }
    }
}

/// サブネットマスクからプレフィックス長を計算
fn calculate_prefix_length(subnet_mask: Ipv4Addr) -> u8 {
    u32::from(subnet_mask).count_ones() as u8
}

/// IPアドレスとサブネットマスクからネットワークアドレスを計算
fn calculate_network_address(ip: Ipv4Addr, subnet_mask: Ipv4Addr) -> Ipv4Addr {
    let ip_u32 = u32::from(ip);
    let mask_u32 = u32::from(subnet_mask);
    let network_u32 = ip_u32 & mask_u32;

    Ipv4Addr::from(network_u32)
}
