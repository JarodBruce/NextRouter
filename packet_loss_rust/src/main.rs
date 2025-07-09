use clap::Parser;
use pcap::{Capture, Device};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::ipv6::Ipv6Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::Packet;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use log::{info, warn, debug};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// ネットワークインターフェース名
    #[arg(short, long)]
    interface: String,
    
    /// 統計出力間隔（秒）
    #[arg(short, long, default_value = "1")]
    stats_interval: u64,
    
    /// 出力ファイル（オプション）
    #[arg(short, long)]
    output: Option<String>,
    
    /// フィルタするポート（オプション）
    #[arg(short, long)]
    port: Option<u16>,
    
    /// 詳細なログを出力
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TcpConnection {
    src_ip: String,
    dst_ip: String,
    src_port: u16,
    dst_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WindowSizeMeasurement {
    timestamp: DateTime<Utc>,
    connection: TcpConnection,
    window_size: u16,
    window_scale: u8,
    effective_window_size: u32,
    direction: String, // "outbound" or "inbound"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConnectionStats {
    connection: TcpConnection,
    measurements: Vec<WindowSizeMeasurement>,
    min_window_size: u32,
    max_window_size: u32,
    avg_window_size: f64,
    window_size_variance: f64,
    last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PacketLossEvent {
    timestamp: DateTime<Utc>,
    connection: TcpConnection,
    expected_seq: u32,
    received_seq: u32,
    gap_size: u32,
    loss_type: PacketLossType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum PacketLossType {
    MissingSequence,    // シーケンス番号の欠損
    DuplicateSequence,  // 重複パケット（再送の可能性）
    OutOfOrder,         // 順序が乱れたパケット
}

#[derive(Debug, Clone)]
struct ConnectionState {
    last_seq: u32,
    last_ack: u32,
    expected_seq: u32,
    packet_count: u64,
    loss_events: Vec<PacketLossEvent>,
    out_of_order_count: u32,
    duplicate_count: u32,
    last_seen: DateTime<Utc>,
}

#[derive(Debug)]
struct GlobalStats {
    total_packets: u64,
    tcp_packets: u64,
    global_tcp_packets: u64,
    connections: HashMap<String, ConnectionStats>,
    connection_states: HashMap<String, ConnectionState>,
    packet_loss_events: Vec<PacketLossEvent>,
    analyses: HashMap<String, ConnectionAnalysis>,
    start_time: Instant,
}

impl Default for GlobalStats {
    fn default() -> Self {
        Self {
            total_packets: 0,
            tcp_packets: 0,
            global_tcp_packets: 0,
            connections: HashMap::new(),
            connection_states: HashMap::new(),
            packet_loss_events: Vec::new(),
            analyses: HashMap::new(),
            start_time: Instant::now(),
        }
    }
}

impl TcpConnection {
    fn key(&self) -> String {
        format!("{}:{}-{}:{}", self.src_ip, self.src_port, self.dst_ip, self.dst_port)
    }
    
    fn reverse_key(&self) -> String {
        format!("{}:{}-{}:{}", self.dst_ip, self.dst_port, self.src_ip, self.src_port)
    }
}

/// IPアドレスがプライベート（ローカル）アドレスかどうかを判定
fn is_private_ip(ip_str: &str) -> bool {
    if let Ok(ip) = ip_str.parse::<IpAddr>() {
        match ip {
            IpAddr::V4(ipv4) => is_private_ipv4(ipv4),
            IpAddr::V6(ipv6) => is_private_ipv6(ipv6),
        }
    } else {
        // パースできない場合は安全のためプライベートと判定
        true
    }
}

/// インターフェースの設定に基づいてIPアドレスがローカルネットワークかどうかを判定
fn is_local_ip_with_interface(ip_str: &str, interface_name: &str) -> bool {
    // まず基本的なプライベートアドレス判定
    if is_private_ip(ip_str) {
        return true;
    }

    // インターフェース情報を取得してローカルネットワーク範囲を確認
    if let Ok(devices) = Device::list() {
        if let Some(device) = devices.into_iter().find(|d| d.name == interface_name) {
            if let Some(addr) = device.addresses.iter().find(|a| a.addr.is_ipv4()) {
                if let (std::net::IpAddr::V4(local_ip), Some(std::net::IpAddr::V4(netmask))) = (addr.addr, addr.netmask) {
                    if let Ok(target_ip) = ip_str.parse::<Ipv4Addr>() {
                        // ローカルネットワークの範囲を計算
                        let local_ip_u32 = u32::from(local_ip);
                        let netmask_u32 = u32::from(netmask);
                        let network_u32 = local_ip_u32 & netmask_u32;
                        
                        let target_ip_u32 = u32::from(target_ip);
                        let target_network_u32 = target_ip_u32 & netmask_u32;
                        
                        // 同じネットワークセグメントかどうか判定
                        if network_u32 == target_network_u32 {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

/// IPv4アドレスがプライベートアドレスかどうかを判定
fn is_private_ipv4(ip: Ipv4Addr) -> bool {
    // RFC 1918 プライベートアドレス範囲
    // 10.0.0.0/8        (10.0.0.0 - 10.255.255.255)
    // 172.16.0.0/12     (172.16.0.0 - 172.31.255.255)
    // 192.168.0.0/16    (192.168.0.0 - 192.168.255.255)
    // 
    // その他のローカルアドレス
    // 127.0.0.0/8       (ループバック)
    // 169.254.0.0/16    (リンクローカル)
    
    let octets = ip.octets();
    
    // 10.0.0.0/8
    if octets[0] == 10 {
        return true;
    }
    
    // 172.16.0.0/12
    if octets[0] == 172 && (octets[1] >= 16 && octets[1] <= 31) {
        return true;
    }
    
    // 192.168.0.0/16
    if octets[0] == 192 && octets[1] == 168 {
        return true;
    }
    
    // 127.0.0.0/8 (ループバック)
    if octets[0] == 127 {
        return true;
    }
    
    // 169.254.0.0/16 (リンクローカル)
    if octets[0] == 169 && octets[1] == 254 {
        return true;
    }
    
    // 0.0.0.0/8 (このネットワーク)
    if octets[0] == 0 {
        return true;
    }
    
    // 224.0.0.0/4 (マルチキャスト)
    if octets[0] >= 224 && octets[0] <= 239 {
        return true;
    }
    
    // 240.0.0.0/4 (実験用)
    if octets[0] >= 240 {
        return true;
    }
    
    false
}

/// IPv6アドレスがプライベートアドレスかどうかを判定
fn is_private_ipv6(ip: Ipv6Addr) -> bool {
    // IPv6のプライベート/ローカルアドレス範囲
    // ::1/128           (ループバック)
    // ::/128            (未指定アドレス)
    // fc00::/7          (Unique Local Address)
    // fe80::/10         (リンクローカル)
    // ff00::/8          (マルチキャスト)
    
    if ip.is_loopback() || ip.is_unspecified() {
        return true;
    }
    
    let segments = ip.segments();
    
    // fc00::/7 (Unique Local Address)
    if (segments[0] & 0xfe00) == 0xfc00 {
        return true;
    }
    
    // fe80::/10 (リンクローカル)
    if (segments[0] & 0xffc0) == 0xfe80 {
        return true;
    }
    
    // ff00::/8 (マルチキャスト)
    if (segments[0] & 0xff00) == 0xff00 {
        return true;
    }
    
    false
}

/// 両方のIPアドレスがグローバルIPかどうかを判定（インターフェース情報を考慮）
fn is_global_connection_with_interface(src_ip: &str, dst_ip: &str, interface_name: &str) -> bool {
    !is_local_ip_with_interface(src_ip, interface_name) && !is_local_ip_with_interface(dst_ip, interface_name)
}

/// 両方のIPアドレスがグローバルIPかどうかを判定（従来の方法）
fn is_global_connection(src_ip: &str, dst_ip: &str) -> bool {
    !is_private_ip(src_ip) && !is_private_ip(dst_ip)
}

/// パケットロスを検出する
fn detect_packet_loss(
    connection: &TcpConnection,
    seq_num: u32,
    ack_num: u32,
    payload_len: u32,
    stats: &mut GlobalStats,
) {
    let connection_key = connection.key();
    
    // 接続状態を取得または作成
    let state = stats.connection_states.entry(connection_key.clone()).or_insert_with(|| {
        ConnectionState {
            last_seq: seq_num,
            last_ack: ack_num,
            expected_seq: seq_num.wrapping_add(payload_len.max(1)), // SYNの場合は1、データがある場合はペイロード長
            packet_count: 0,
            loss_events: Vec::new(),
            out_of_order_count: 0,
            duplicate_count: 0,
            last_seen: Utc::now(),
        }
    });
    
    state.packet_count += 1;
    state.last_seen = Utc::now();
    
    // ペイロードがある場合のみシーケンス番号分析を行う
    if payload_len > 0 {
        // パケットロス検出ロジック
        if seq_num == state.expected_seq {
            // 期待通りのシーケンス番号
            state.last_seq = seq_num;
            state.expected_seq = seq_num.wrapping_add(payload_len);
        } else if seq_num > state.expected_seq {
            // シーケンス番号にギャップがある = パケットロスの可能性
            let gap_size = seq_num.wrapping_sub(state.expected_seq);
            
            // 小さなギャップは無視（TCP の実装による誤差の可能性）
            if gap_size > 0 && gap_size < 1000000 { // 1MB以下のギャップのみ検出
                let loss_event = PacketLossEvent {
                    timestamp: Utc::now(),
                    connection: connection.clone(),
                    expected_seq: state.expected_seq,
                    received_seq: seq_num,
                    gap_size,
                    loss_type: PacketLossType::MissingSequence,
                };
                
                state.loss_events.push(loss_event.clone());
                stats.packet_loss_events.push(loss_event);
            }
            
            state.last_seq = seq_num;
            state.expected_seq = seq_num.wrapping_add(payload_len);
        } else if seq_num < state.expected_seq {
            // 過去のシーケンス番号 = 重複パケットまたは順序乱れ
            if seq_num == state.last_seq {
                // 完全に同じシーケンス番号 = 重複パケット（再送）
                state.duplicate_count += 1;
                
                let loss_event = PacketLossEvent {
                    timestamp: Utc::now(),
                    connection: connection.clone(),
                    expected_seq: state.expected_seq,
                    received_seq: seq_num,
                    gap_size: 0,
                    loss_type: PacketLossType::DuplicateSequence,
                };
                
                state.loss_events.push(loss_event.clone());
                stats.packet_loss_events.push(loss_event);
            } else {
                // 順序乱れ
                state.out_of_order_count += 1;
                
                let loss_event = PacketLossEvent {
                    timestamp: Utc::now(),
                    connection: connection.clone(),
                    expected_seq: state.expected_seq,
                    received_seq: seq_num,
                    gap_size: state.expected_seq.wrapping_sub(seq_num),
                    loss_type: PacketLossType::OutOfOrder,
                };
                
                state.loss_events.push(loss_event.clone());
                stats.packet_loss_events.push(loss_event);

            }
        }
    }
    
    // ACK番号の更新
    if ack_num > state.last_ack {
        state.last_ack = ack_num;
    }
}

fn get_window_scale_from_options(_tcp_packet: &TcpPacket) -> u8 {
    // TCP オプションから Window Scale を取得
    // 簡単な実装のため、デフォルトで0を返す
    // 実際の実装では TCP オプションを解析する必要がある
    0
}

fn calculate_effective_window_size(window_size: u16, window_scale: u8) -> u32 {
    (window_size as u32) << window_scale
}

fn process_tcp_packet(
    tcp_packet: &TcpPacket,
    src_ip: String,
    dst_ip: String,
    stats: &Arc<Mutex<GlobalStats>>,
    port_filter: Option<u16>,
    interface_name: &str,
) {
    let src_port = tcp_packet.get_source();
    let dst_port = tcp_packet.get_destination();
    
    // ポートフィルタリング
    if let Some(filter_port) = port_filter {
        if src_port != filter_port && dst_port != filter_port {
            return;
        }
    }
    
    let window_size = tcp_packet.get_window();
    let window_scale = get_window_scale_from_options(tcp_packet);
    let effective_window_size = calculate_effective_window_size(window_size, window_scale);
    
    // TCP シーケンス番号とACK番号を取得
    let seq_num = tcp_packet.get_sequence();
    let ack_num = tcp_packet.get_acknowledgement();
    let payload_len = tcp_packet.payload().len() as u32;
    
    let connection = TcpConnection {
        src_ip: src_ip.clone(),
        dst_ip: dst_ip.clone(),
        src_port,
        dst_port,
    };
    
    let measurement = WindowSizeMeasurement {
        timestamp: Utc::now(),
        connection: connection.clone(),
        window_size,
        window_scale,
        effective_window_size,
        direction: "outbound".to_string(), // 簡単な実装
    };
    
    let mut stats_guard = stats.lock().unwrap();
    stats_guard.tcp_packets += 1;
    
    // インターフェース情報を考慮したグローバル接続判定を使用
    if is_global_connection_with_interface(&src_ip, &dst_ip, interface_name) {
        stats_guard.global_tcp_packets += 1;
    }
    
    let connection_key = connection.key();
    let reverse_key = connection.reverse_key();
    
    // パケットロス検出
    detect_packet_loss(&connection, seq_num, ack_num, payload_len, &mut stats_guard);
    
    // 既存の接続を探す（双方向を考慮）
    let key_to_use = if stats_guard.connections.contains_key(&connection_key) {
        connection_key
    } else if stats_guard.connections.contains_key(&reverse_key) {
        reverse_key
    } else {
        connection_key
    };
    
    let conn_stats = stats_guard.connections.entry(key_to_use).or_insert_with(|| {
        ConnectionStats {
            connection: connection.clone(),
            measurements: Vec::new(),
            min_window_size: effective_window_size,
            max_window_size: effective_window_size,
            avg_window_size: effective_window_size as f64,
            window_size_variance: 0.0,
            last_seen: Utc::now(),
        }
    });
    
    // 統計を更新
    conn_stats.measurements.push(measurement);
    conn_stats.min_window_size = conn_stats.min_window_size.min(effective_window_size);
    conn_stats.max_window_size = conn_stats.max_window_size.max(effective_window_size);
    conn_stats.last_seen = Utc::now();
    
    // 平均と分散を計算
    let window_sizes: Vec<f64> = conn_stats.measurements
        .iter()
        .map(|m| m.effective_window_size as f64)
        .collect();
    
    if !window_sizes.is_empty() {
        conn_stats.avg_window_size = window_sizes.iter().sum::<f64>() / window_sizes.len() as f64;
        
        if window_sizes.len() > 1 {
            let variance = window_sizes
                .iter()
                .map(|x| (x - conn_stats.avg_window_size).powi(2))
                .sum::<f64>() / (window_sizes.len() - 1) as f64;
            conn_stats.window_size_variance = variance;
        }
    }
}

fn process_packet(packet_data: &[u8], stats: &Arc<Mutex<GlobalStats>>, port_filter: Option<u16>, interface_name: &str) {
    let mut stats_guard = stats.lock().unwrap();
    stats_guard.total_packets += 1;
    drop(stats_guard);
    
    if let Some(ethernet) = EthernetPacket::new(packet_data) {
        match ethernet.get_ethertype() {
            EtherTypes::Ipv4 => {
                if let Some(ipv4) = Ipv4Packet::new(ethernet.payload()) {
                    if ipv4.get_next_level_protocol() == IpNextHeaderProtocols::Tcp {
                        if let Some(tcp) = TcpPacket::new(ipv4.payload()) {
                            let src_ip = ipv4.get_source().to_string();
                            let dst_ip = ipv4.get_destination().to_string();
                            process_tcp_packet(&tcp, src_ip, dst_ip, stats, port_filter, interface_name);
                        }
                    }
                }
            }
            EtherTypes::Ipv6 => {
                if let Some(ipv6) = Ipv6Packet::new(ethernet.payload()) {
                    if ipv6.get_next_header() == IpNextHeaderProtocols::Tcp {
                        if let Some(tcp) = TcpPacket::new(ipv6.payload()) {
                            let src_ip = ipv6.get_source().to_string();
                            let dst_ip = ipv6.get_destination().to_string();
                            process_tcp_packet(&tcp, src_ip, dst_ip, stats, port_filter, interface_name);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn perform_detailed_analysis(stats: &Arc<Mutex<GlobalStats>>) {
    let mut stats_guard = stats.lock().unwrap();
    
    // Clone the connections data to avoid borrowing conflicts
    let connections_data: Vec<(String, Vec<u32>)> = stats_guard.connections
        .iter()
        .filter(|(_, conn_stats)| conn_stats.measurements.len() >= 10)
        .map(|(key, conn_stats)| {
            let window_sizes: Vec<u32> = conn_stats.measurements
                .iter()
                .map(|m| m.effective_window_size)
                .collect();
            (key.clone(), window_sizes)
        })
        .collect();
    
    // Now perform analysis on the cloned data
    for (key, window_sizes) in connections_data {
        // Simple RTT estimation (placeholder - in real implementation, this would be measured)
        let estimated_rtt = vec![50.0; window_sizes.len()]; // 50ms default RTT
        
        let mut analysis = ConnectionAnalysis::new(key.clone());
        analysis.complete_analysis(&window_sizes, &estimated_rtt);
        
        stats_guard.analyses.insert(key, analysis);
    }
}

fn print_statistics(stats: &Arc<Mutex<GlobalStats>>) {
    let stats_guard = stats.lock().unwrap();
    let elapsed = stats_guard.start_time.elapsed();
    
    // パケットロス統計を表示
    if !stats_guard.packet_loss_events.is_empty() {
        let mut missing_count = 0;
        let mut duplicate_count = 0;
        let mut out_of_order_count = 0;
        
        for event in &stats_guard.packet_loss_events {
            match event.loss_type {
                PacketLossType::MissingSequence => missing_count += 1,
                PacketLossType::DuplicateSequence => duplicate_count += 1,
                PacketLossType::OutOfOrder => out_of_order_count += 1,
            }
        }
        
        println!("\n--- パケットロス詳細 ---");
        println!("  パケット欠損: {} 回", missing_count);
        println!("  重複パケット: {} 回", duplicate_count);
        println!("  順序乱れ: {} 回", out_of_order_count);
        
        if stats_guard.global_tcp_packets > 0 {
            let loss_rate = (missing_count as f64 / stats_guard.global_tcp_packets as f64) * 100.0;
            println!("  推定パケットロス率: {:.4}%", loss_rate);
        }
    }
    
    // 接続別統計を表示
    for (key, conn_stats) in &stats_guard.connections {
        if conn_stats.measurements.len() > 5 { // 十分なサンプルがある接続のみ表示
            println!("\n--- 接続: {} ---", key);
            println!("  測定回数: {}", conn_stats.measurements.len());
            println!("  最小ウィンドウサイズ: {} bytes", conn_stats.min_window_size);
            println!("  最大ウィンドウサイズ: {} bytes", conn_stats.max_window_size);
            println!("  平均ウィンドウサイズ: {:.0} bytes", conn_stats.avg_window_size);
            println!("  標準偏差: {:.0} bytes", conn_stats.window_size_variance.sqrt());
            
            // この接続のパケットロス情報を表示
            if let Some(conn_state) = stats_guard.connection_states.get(key) {
                println!("  パケット総数: {}", conn_state.packet_count);
                println!("  パケットロスイベント: {} 回", conn_state.loss_events.len());
                println!("  重複パケット: {} 回", conn_state.duplicate_count);
                println!("  順序乱れ: {} 回", conn_state.out_of_order_count);
                
                if conn_state.packet_count > 0 && !conn_state.loss_events.is_empty() {
                    let connection_loss_rate = (conn_state.loss_events.len() as f64 / conn_state.packet_count as f64) * 100.0;
                    println!("  接続パケットロス率: {:.4}%", connection_loss_rate);
                }
            }
            
            // 推定帯域幅（簡単な計算）
            let max_window_kb = conn_stats.max_window_size as f64 / 1024.0;
            println!("  推定最大帯域幅 (Window based): {:.2} KB/s", max_window_kb);
            
            println!("  最後の観測: {}", conn_stats.last_seen.format("%Y-%m-%d %H:%M:%S UTC"));
        }
    }
}

// === Analysis module content (previously in analysis.rs) ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthEstimate {
    pub timestamp: DateTime<Utc>,
    pub window_size: u32,
    pub estimated_rtt_ms: f64,
    pub estimated_bandwidth_bps: f64,
    pub confidence: f64, // 0.0 - 1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionAnalysis {
    pub connection_key: String,
    pub total_measurements: usize,
    pub analysis_period_seconds: f64,
    
    // Window size statistics
    pub window_stats: WindowStatistics,
    
    // Bandwidth estimates
    pub bandwidth_estimates: Vec<BandwidthEstimate>,
    pub peak_bandwidth_bps: f64,
    pub average_bandwidth_bps: f64,
    
    // Flow control analysis
    pub zero_window_events: usize,
    pub window_scaling_factor: u8,
    pub congestion_control_events: usize,
    
    // Performance indicators
    pub bottleneck_type: BottleneckType,
    pub performance_score: f64, // 0.0 - 100.0
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowStatistics {
    pub min: u32,
    pub max: u32,
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
    pub percentile_95: f64,
    pub percentile_99: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BottleneckType {
    NetworkBandwidth,
    ReceiverBuffer,
    ApplicationProcessing,
    CongestionControl,
    Unknown,
}

impl ConnectionAnalysis {
    pub fn new(connection_key: String) -> Self {
        Self {
            connection_key,
            total_measurements: 0,
            analysis_period_seconds: 0.0,
            window_stats: WindowStatistics::default(),
            bandwidth_estimates: Vec::new(),
            peak_bandwidth_bps: 0.0,
            average_bandwidth_bps: 0.0,
            zero_window_events: 0,
            window_scaling_factor: 0,
            congestion_control_events: 0,
            bottleneck_type: BottleneckType::Unknown,
            performance_score: 0.0,
            recommendations: Vec::new(),
        }
    }
    
    pub fn analyze_window_sizes(&mut self, window_sizes: &[u32]) {
        if window_sizes.is_empty() {
            return;
        }
        
        self.total_measurements = window_sizes.len();
        
        // Basic statistics
        let min = *window_sizes.iter().min().unwrap();
        let max = *window_sizes.iter().max().unwrap();
        let sum: u32 = window_sizes.iter().sum();
        let mean = sum as f64 / window_sizes.len() as f64;
        
        // Calculate median and percentiles
        let mut sorted_sizes = window_sizes.to_vec();
        sorted_sizes.sort_unstable();
        
        let median = if sorted_sizes.len() % 2 == 0 {
            let mid = sorted_sizes.len() / 2;
            (sorted_sizes[mid - 1] + sorted_sizes[mid]) as f64 / 2.0
        } else {
            sorted_sizes[sorted_sizes.len() / 2] as f64
        };
        
        let percentile_95 = sorted_sizes[(sorted_sizes.len() as f64 * 0.95) as usize] as f64;
        let percentile_99 = sorted_sizes[(sorted_sizes.len() as f64 * 0.99) as usize] as f64;
        
        // Calculate standard deviation
        let variance = window_sizes
            .iter()
            .map(|&x| (x as f64 - mean).powi(2))
            .sum::<f64>() / (window_sizes.len() - 1) as f64;
        let std_dev = variance.sqrt();
        
        self.window_stats = WindowStatistics {
            min,
            max,
            mean,
            median,
            std_dev,
            percentile_95,
            percentile_99,
        };
        
        // Count zero window events
        self.zero_window_events = window_sizes.iter().filter(|&&x| x == 0).count();
        
        // Detect congestion control events (significant window size decreases)
        self.detect_congestion_events(window_sizes);
    }
    
    fn detect_congestion_events(&mut self, window_sizes: &[u32]) {
        let mut congestion_events = 0;
        let threshold = 0.5; // 50% decrease threshold
        
        for window in window_sizes.windows(2) {
            if window[0] > 0 && (window[1] as f64) < (window[0] as f64 * threshold) {
                congestion_events += 1;
            }
        }
        
        self.congestion_control_events = congestion_events;
    }
    
    pub fn estimate_bandwidth(&mut self, rtt_estimates: &[f64]) {
        if self.window_stats.max == 0 || rtt_estimates.is_empty() {
            return;
        }
        
        let mut bandwidth_estimates = Vec::new();
        let avg_rtt = rtt_estimates.iter().sum::<f64>() / rtt_estimates.len() as f64;
        
        // Simple bandwidth estimation: Window Size / RTT
        let max_bandwidth = (self.window_stats.max as f64 * 8.0) / (avg_rtt / 1000.0); // bps
        let avg_bandwidth = (self.window_stats.mean * 8.0) / (avg_rtt / 1000.0); // bps
        
        self.peak_bandwidth_bps = max_bandwidth;
        self.average_bandwidth_bps = avg_bandwidth;
        
        // Create bandwidth estimate
        let estimate = BandwidthEstimate {
            timestamp: Utc::now(),
            window_size: self.window_stats.max,
            estimated_rtt_ms: avg_rtt,
            estimated_bandwidth_bps: max_bandwidth,
            confidence: self.calculate_confidence(),
        };
        
        bandwidth_estimates.push(estimate);
        self.bandwidth_estimates = bandwidth_estimates;
    }
    
    fn calculate_confidence(&self) -> f64 {
        // Confidence based on number of measurements and variance
        let measurement_confidence = (self.total_measurements as f64 / 100.0).min(1.0);
        let variance_confidence = 1.0 - (self.window_stats.std_dev / self.window_stats.mean).min(1.0);
        
        (measurement_confidence + variance_confidence) / 2.0
    }
    
    pub fn determine_bottleneck(&mut self) {
        // Analyze patterns to determine bottleneck type
        let cv = self.window_stats.std_dev / self.window_stats.mean; // Coefficient of variation
        
        self.bottleneck_type = if self.zero_window_events > self.total_measurements / 10 {
            BottleneckType::ApplicationProcessing
        } else if cv > 0.5 && self.congestion_control_events > self.total_measurements / 20 {
            BottleneckType::CongestionControl
        } else if self.window_stats.max < 8192 {
            BottleneckType::ReceiverBuffer
        } else if cv < 0.2 && self.window_stats.mean > 32768.0 {
            BottleneckType::NetworkBandwidth
        } else {
            BottleneckType::Unknown
        };
    }
    
    pub fn calculate_performance_score(&mut self) {
        let mut score = 100.0;
        
        // Reduce score for zero window events
        if self.zero_window_events > 0 {
            score -= (self.zero_window_events as f64 / self.total_measurements as f64) * 30.0;
        }
        
        // Reduce score for high congestion events
        if self.congestion_control_events > 0 {
            score -= (self.congestion_control_events as f64 / self.total_measurements as f64) * 20.0;
        }
        
        // Reduce score for small window sizes
        if self.window_stats.mean < 16384.0 {
            score -= 25.0;
        }
        
        // Reduce score for high variance
        let cv = self.window_stats.std_dev / self.window_stats.mean;
        if cv > 0.5 {
            score -= cv * 20.0;
        }
        
        self.performance_score = score.max(0.0).min(100.0);
    }
    
    pub fn generate_recommendations(&mut self) {
        let mut recommendations = Vec::new();
        
        match self.bottleneck_type {
            BottleneckType::ApplicationProcessing => {
                recommendations.push("アプリケーションの処理速度を向上させることを検討してください".to_string());
                recommendations.push("受信バッファのサイズを増やすことを検討してください".to_string());
            }
            BottleneckType::ReceiverBuffer => {
                recommendations.push("TCP受信バッファサイズ (net.core.rmem_max) を増やしてください".to_string());
                recommendations.push("アプリケーションの読み込み頻度を増やしてください".to_string());
            }
            BottleneckType::NetworkBandwidth => {
                recommendations.push("ネットワーク帯域幅がボトルネックになっている可能性があります".to_string());
                recommendations.push("ネットワーク経路の最適化を検討してください".to_string());
            }
            BottleneckType::CongestionControl => {
                recommendations.push("ネットワーク輻輳が発生しています".to_string());
                recommendations.push("TCP輻輳制御アルゴリズムの調整を検討してください".to_string());
            }
            BottleneckType::Unknown => {
                recommendations.push("より長期間の監視でパターンを分析してください".to_string());
            }
        }
        self.recommendations = recommendations;
    }
    
    pub fn complete_analysis(&mut self, window_sizes: &[u32], rtt_estimates: &[f64]) {
        self.analyze_window_sizes(window_sizes);
        self.estimate_bandwidth(rtt_estimates);
        self.determine_bottleneck();
        self.calculate_performance_score();
        self.generate_recommendations();
    }
}

impl Default for WindowStatistics {
    fn default() -> Self {
        Self {
            min: 0,
            max: 0,
            mean: 0.0,
            median: 0.0,
            std_dev: 0.0,
            percentile_95: 0.0,
            percentile_99: 0.0,
        }
    }
}

// === End of Analysis module content ===

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // ログレベルの設定
    if args.verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }
    
    info!("TCP Window Size Monitor & パケットロス検出 を開始します");
    // 自分のIPアドレスとサブネットマスクを取得
    if let Some(device) = Device::list()?.into_iter().find(|d| d.name == args.interface) {
        if let Some(addr) = device.addresses.iter().find(|a| a.addr.is_ipv4()) {
            if let (std::net::IpAddr::V4(ip), Some(std::net::IpAddr::V4(netmask))) = (addr.addr, addr.netmask) {
                info!("自分のIPアドレス: {}", ip);
                info!("サブネットマスク: {}", netmask);

                let ip_u32 = u32::from(ip);
                let netmask_u32 = u32::from(netmask);
                let network_u32 = ip_u32 & netmask_u32;
                let broadcast_u32 = network_u32 | !netmask_u32;

                let network_ip = Ipv4Addr::from(network_u32);
                let broadcast_ip = Ipv4Addr::from(broadcast_u32);
                
                info!("IPアドレス範囲: {} - {}", network_ip, broadcast_ip);
            }
        }
    }
    info!("インターフェース: {}", args.interface);
    info!("対象: グローバルIP間のTCP通信のみ");
    
    // pcap デバイスの取得
    let device = Device::list()?
        .into_iter()
        .find(|d| d.name == args.interface)
        .ok_or_else(|| format!("インターフェース '{}' が見つかりません", args.interface))?;
    
    info!("デバイス: {} を開いています", device.name);
    
    // キャプチャの開始
    let mut cap = Capture::from_device(device)?
        .promisc(true)
        .snaplen(65536)
        .timeout(1000)
        .open()?;
    
    // TCPフィルタを設定
    let filter = if let Some(port) = args.port {
        format!("tcp and port {}", port)
    } else {
        "tcp".to_string()
    };
    
    cap.filter(&filter, true)?;
    info!("フィルタを設定しました: {}", filter);
    
    let stats = Arc::new(Mutex::new(GlobalStats {
        start_time: Instant::now(),
        ..Default::default()
    }));
    
    let stats_clone = Arc::clone(&stats);
    let stats_interval = args.stats_interval;
    
    // 統計表示用のタスク
    let stats_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(stats_interval));
        
        loop {
            interval.tick().await;
            
            // 詳細分析を実行
            perform_detailed_analysis(&stats_clone);
            
            print_statistics(&stats_clone);
        }
    });
    
    // パケットキャプチャのメインループ
    info!("パケットキャプチャを開始します...");
    
    loop {
        match cap.next_packet() {
            Ok(packet) => {
                process_packet(packet.data, &stats, args.port, &args.interface);
            }
            Err(pcap::Error::TimeoutExpired) => {
                // タイムアウトは正常、続行
                continue;
            }
            Err(e) => {
                warn!("パケットキャプチャエラー: {}", e);
                continue;
            }
        }
    }
    
    // Note: このループは無制限に実行されます (Ctrl+Cで停止)
    // stats_task.abort(); は到達しません
    
    // 最終統計を表示 (実際には到達しません)
    println!("\n=== 最終統計（グローバルIP間通信のみ） ===");
    perform_detailed_analysis(&stats);
    print_statistics(&stats);
    
    info!("監視を終了しました");
    Ok(())
}
