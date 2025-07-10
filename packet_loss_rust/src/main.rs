use clap::Parser;
use pcap::{Capture, Device};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::Packet;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::net::Ipv4Addr;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use log::{info, warn};
use prometheus::{Counter, Gauge, Histogram, Registry, TextEncoder};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};
use std::convert::Infallible;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// ネットワークインターフェース名
    #[arg(short, long)]
    interface: String,
    
    /// 統計出力間隔（秒）
    #[arg(short, long, default_value = "1")]
    stats_interval: u64,
    
    /// 詳細なログを出力
    #[arg(short, long)]
    verbose: bool,
    
    /// Prometheusメトリクス用のHTTPポート
    #[arg(short, long, default_value = "9090")]
    prometheus_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TcpConnection {
    src_ip: String,
    dst_ip: String,
    src_port: u16,
    dst_port: u16,
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
struct PrometheusMetrics {
    registry: Registry,
    // カウンターメトリクス
    total_packets_counter: Counter,
    tcp_packets_counter: Counter,
    global_tcp_packets_counter: Counter,
    packet_loss_missing_counter: Counter,
    packet_loss_duplicate_counter: Counter,
    packet_loss_out_of_order_counter: Counter,
    window_shrink_counter: Counter,
    
    // ゲージメトリクス
    active_connections_gauge: Gauge,
    current_window_size_gauge: Gauge,
    
    // ヒストグラム
    packet_loss_gap_histogram: Histogram,
}

impl PrometheusMetrics {
    fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();
        
        let total_packets_counter = Counter::new(
            "tcp_monitor_total_packets",
            "Total number of packets processed"
        )?;
        
        let tcp_packets_counter = Counter::new(
            "tcp_monitor_tcp_packets",
            "Total number of TCP packets processed"
        )?;
        
        let global_tcp_packets_counter = Counter::new(
            "tcp_monitor_global_tcp_packets",
            "Total number of global TCP packets processed"
        )?;
        
        let packet_loss_missing_counter = Counter::new(
            "tcp_monitor_packet_loss_missing",
            "Number of missing sequence packet loss events"
        )?;
        
        let packet_loss_duplicate_counter = Counter::new(
            "tcp_monitor_packet_loss_duplicate",
            "Number of duplicate packet loss events"
        )?;
        
        let packet_loss_out_of_order_counter = Counter::new(
            "tcp_monitor_packet_loss_out_of_order",
            "Number of out-of-order packet loss events"
        )?;
        
        let window_shrink_counter = Counter::new(
            "tcp_monitor_window_shrink",
            "Number of TCP window shrink events"
        )?;
        
        let active_connections_gauge = Gauge::new(
            "tcp_monitor_active_connections",
            "Number of active TCP connections"
        )?;
        
        let current_window_size_gauge = Gauge::new(
            "tcp_monitor_current_window_size",
            "Current TCP window size"
        )?;
        
        let packet_loss_gap_histogram = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "tcp_monitor_packet_loss_gap",
                "Distribution of packet loss gap sizes"
            ).buckets(vec![1.0, 5.0, 10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0])
        )?;
        
        // メトリクスを登録
        registry.register(Box::new(total_packets_counter.clone()))?;
        registry.register(Box::new(tcp_packets_counter.clone()))?;
        registry.register(Box::new(global_tcp_packets_counter.clone()))?;
        registry.register(Box::new(packet_loss_missing_counter.clone()))?;
        registry.register(Box::new(packet_loss_duplicate_counter.clone()))?;
        registry.register(Box::new(packet_loss_out_of_order_counter.clone()))?;
        registry.register(Box::new(window_shrink_counter.clone()))?;
        registry.register(Box::new(active_connections_gauge.clone()))?;
        registry.register(Box::new(current_window_size_gauge.clone()))?;
        registry.register(Box::new(packet_loss_gap_histogram.clone()))?;
        
        Ok(PrometheusMetrics {
            registry,
            total_packets_counter,
            tcp_packets_counter,
            global_tcp_packets_counter,
            packet_loss_missing_counter,
            packet_loss_duplicate_counter,
            packet_loss_out_of_order_counter,
            window_shrink_counter,
            active_connections_gauge,
            current_window_size_gauge,
            packet_loss_gap_histogram,
        })
    }
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
    last_window_size: u16,
}

#[derive(Debug)]
struct GlobalStats {
    total_packets: u64,
    tcp_packets: u64,
    global_tcp_packets: u64,
    connection_states: HashMap<String, ConnectionState>,
    packet_loss_events: Vec<PacketLossEvent>,
    window_shrink_events: u32,
    start_time: Instant,
    last_reset_time: Instant,
    prometheus_metrics: PrometheusMetrics,
}

impl Default for GlobalStats {
    fn default() -> Self {
        let now = Instant::now();
        let prometheus_metrics = PrometheusMetrics::new().expect("Failed to create Prometheus metrics");
        
        Self {
            total_packets: 0,
            tcp_packets: 0,
            global_tcp_packets: 0,
            connection_states: HashMap::new(),
            packet_loss_events: Vec::new(),
            window_shrink_events: 0,
            start_time: now,
            last_reset_time: now,
            prometheus_metrics,
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
    if let Ok(ip) = ip_str.parse::<Ipv4Addr>() {
        is_private_ipv4(ip)
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
    ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_multicast()
        || ip.is_unspecified()
        || ip.is_documentation()
}

/// 両方のIPアドレスがグローバルIPかどうかを判定（インターフェース情報を考慮）
fn is_global_connection_with_interface(src_ip: &str, dst_ip: &str, interface_name: &str) -> bool {
    !is_local_ip_with_interface(src_ip, interface_name) && !is_local_ip_with_interface(dst_ip, interface_name)
}

/// 両方のIPアドレスがグローバルIPかどうかを判定（従来の方法）
fn is_global_connection(src_ip: &str, dst_ip: &str) -> bool {
    !is_private_ip(src_ip) && !is_private_ip(dst_ip)
}

/// パケットロスとウィンドウサイズの縮小を検出する
fn detect_packet_loss_and_window_shrink(
    connection: &TcpConnection,
    seq_num: u32,
    ack_num: u32,
    payload_len: u32,
    window_size: u16,
    stats: &mut GlobalStats,
) {
    let connection_key = connection.key();
    
    // 接続状態を取得または作成
    let state = stats.connection_states.entry(connection_key.clone()).or_insert_with(|| {
        ConnectionState {
            last_seq: seq_num,
            last_ack: ack_num,
            expected_seq: seq_num.wrapping_add(payload_len.max(1)),
            packet_count: 0,
            loss_events: Vec::new(),
            out_of_order_count: 0,
            duplicate_count: 0,
            last_seen: Utc::now(),
            last_window_size: window_size,
        }
    });
    
    state.packet_count += 1;
    state.last_seen = Utc::now();
    
    // ウィンドウサイズの縮小検出
    if state.last_window_size > 0 && window_size < state.last_window_size {
        let shrink_ratio = (state.last_window_size - window_size) as f64 / state.last_window_size as f64;
        if shrink_ratio > 0.3 { // 30%以上の縮小を検出
            stats.window_shrink_events += 1;
            stats.prometheus_metrics.window_shrink_counter.inc();
        }
    }
    state.last_window_size = window_size;
    
    // 現在のウィンドウサイズを更新
    stats.prometheus_metrics.current_window_size_gauge.set(window_size as f64);
    
    // ペイロードがある場合のみシーケンス番号分析を行う
    if payload_len > 0 {
        if seq_num == state.expected_seq {
            state.last_seq = seq_num;
            state.expected_seq = seq_num.wrapping_add(payload_len);
        } else if seq_num > state.expected_seq {
            let gap_size = seq_num.wrapping_sub(state.expected_seq);
            
            if gap_size > 0 && gap_size < 1000000 {
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
                
                // Prometheusメトリクスを更新
                stats.prometheus_metrics.packet_loss_missing_counter.inc();
                stats.prometheus_metrics.packet_loss_gap_histogram.observe(gap_size as f64);
            }
            
            state.last_seq = seq_num;
            state.expected_seq = seq_num.wrapping_add(payload_len);
        } else if seq_num < state.expected_seq {
            if seq_num == state.last_seq {
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
                
                // Prometheusメトリクスを更新
                stats.prometheus_metrics.packet_loss_duplicate_counter.inc();
            } else {
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
                
                // Prometheusメトリクスを更新
                stats.prometheus_metrics.packet_loss_out_of_order_counter.inc();
            }
        }
    }
    
    if ack_num > state.last_ack {
        state.last_ack = ack_num;
    }
    
    // 最後にアクティブ接続数を更新
    let active_connections_count = stats.connection_states.len();
    stats.prometheus_metrics.active_connections_gauge.set(active_connections_count as f64);
}

fn process_tcp_packet(
    tcp_packet: &TcpPacket,
    src_ip: String,
    dst_ip: String,
    stats: &Arc<Mutex<GlobalStats>>,
    interface_name: &str,
) {
    let src_port = tcp_packet.get_source();
    let dst_port = tcp_packet.get_destination();
    let window_size = tcp_packet.get_window();
    
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
    
    let mut stats_guard = stats.lock().unwrap();
    stats_guard.tcp_packets += 1;
    stats_guard.prometheus_metrics.tcp_packets_counter.inc();
    
    // インターフェース情報を考慮したグローバル接続判定を使用
    if is_global_connection_with_interface(&src_ip, &dst_ip, interface_name) {
        stats_guard.global_tcp_packets += 1;
        stats_guard.prometheus_metrics.global_tcp_packets_counter.inc();
    }
    
    // パケットロス検出とウィンドウサイズの縮小検出
    detect_packet_loss_and_window_shrink(&connection, seq_num, ack_num, payload_len, window_size, &mut stats_guard);
}

fn process_packet(packet_data: &[u8], stats: &Arc<Mutex<GlobalStats>>, interface_name: &str) {
    let mut stats_guard = stats.lock().unwrap();
    stats_guard.total_packets += 1;
    stats_guard.prometheus_metrics.total_packets_counter.inc();
    drop(stats_guard);
    
    if let Some(ethernet) = EthernetPacket::new(packet_data) {
        if ethernet.get_ethertype() == EtherTypes::Ipv4 {
            if let Some(ipv4) = Ipv4Packet::new(ethernet.payload()) {
                if ipv4.get_next_level_protocol() == IpNextHeaderProtocols::Tcp {
                    if let Some(tcp) = TcpPacket::new(ipv4.payload()) {
                        let src_ip = ipv4.get_source().to_string();
                        let dst_ip = ipv4.get_destination().to_string();
                        process_tcp_packet(&tcp, src_ip, dst_ip, stats, interface_name);
                    }
                }
            }
        }
    }
}

fn print_statistics(stats: &Arc<Mutex<GlobalStats>>) {
    let mut stats_guard = stats.lock().unwrap();
    let current_time = Instant::now();
    
    // 1秒間のパケットロス統計をカウント
    let mut missing_count = 0;
    let mut duplicate_count = 0;
    let mut out_of_order_count = 0;
    
    // 最後のリセット時刻以降のイベントのみカウント
    let reset_time = stats_guard.last_reset_time;
    for event in &stats_guard.packet_loss_events {
        let event_elapsed = current_time.duration_since(stats_guard.start_time);
        let event_time = stats_guard.start_time + Duration::from_secs(event_elapsed.as_secs());
        
        if event_time >= reset_time {
            match event.loss_type {
                PacketLossType::MissingSequence => missing_count += 1,
                PacketLossType::DuplicateSequence => duplicate_count += 1,
                PacketLossType::OutOfOrder => out_of_order_count += 1,
            }
        }
    }
    
    // 1秒間の統計を表示
    println!("\n=== 1秒間の統計 ===");
    println!("時刻: {}", Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
    println!("パケット欠損: {} 回", missing_count);
    println!("重複パケット: {} 回", duplicate_count);
    println!("順序乱れ: {} 回", out_of_order_count);
    println!("ウィンドウサイズ縮小: {} 回", stats_guard.window_shrink_events);
    println!("総パケットロス: {} 回", missing_count + duplicate_count + out_of_order_count); 
    
    // 統計をリセット
    stats_guard.packet_loss_events.clear();
    stats_guard.window_shrink_events = 0;
    stats_guard.last_reset_time = current_time;
}

// Prometheusメトリクスを提供するHTTPサーバー
async fn metrics_handler(
    _req: Request<Body>,
    stats: Arc<Mutex<GlobalStats>>,
) -> Result<Response<Body>, Infallible> {
    let stats_guard = stats.lock().unwrap();
    let encoder = TextEncoder::new();
    let metric_families = stats_guard.prometheus_metrics.registry.gather();
    
    match encoder.encode_to_string(&metric_families) {
        Ok(metrics_string) => {
            let response = Response::builder()
                .header("Content-Type", "text/plain; version=0.0.4")
                .body(Body::from(metrics_string))
                .unwrap();
            Ok(response)
        }
        Err(_) => {
            let response = Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Error encoding metrics"))
                .unwrap();
            Ok(response)
        }
    }
}

async fn start_prometheus_server(port: u16, stats: Arc<Mutex<GlobalStats>>) -> Result<(), Box<dyn std::error::Error>> {
    let addr = ([0, 0, 0, 0], port).into();
    
    let make_svc = make_service_fn(move |_conn| {
        let stats = Arc::clone(&stats);
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                metrics_handler(req, Arc::clone(&stats))
            }))
        }
    });
    
    let server = Server::bind(&addr).serve(make_svc);
    
    info!("Prometheusメトリクスサーバーを開始しました: http://0.0.0.0:{}/metrics", port);
    
    if let Err(e) = server.await {
        warn!("Prometheusサーバーエラー: {}", e);
    }
    
    Ok(())
}

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
    let filter = "tcp".to_string();
    
    cap.filter(&filter, true)?;
    info!("フィルタを設定しました: {}", filter);
    
    let stats = Arc::new(Mutex::new(GlobalStats {
        start_time: Instant::now(),
        ..Default::default()
    }));
    
    let stats_clone_for_stats = Arc::clone(&stats);
    let stats_interval = args.stats_interval;
    let prometheus_port = args.prometheus_port;
    
    // Prometheusメトリクスサーバーの起動
    let prometheus_stats = Arc::clone(&stats);
    tokio::spawn(async move {
        if let Err(e) = start_prometheus_server(prometheus_port, prometheus_stats).await {
            warn!("Prometheusサーバーの起動に失敗しました: {}", e);
        }
    });
    
    // 統計表示用のタスク
    let _stats_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(stats_interval));
        
        loop {
            interval.tick().await;
            print_statistics(&stats_clone_for_stats);
        }
    });
    
    // パケットキャプチャのメインループ
    info!("パケットキャプチャを開始します...");
    
    loop {
        match cap.next_packet() {
            Ok(packet) => {
                process_packet(packet.data, &stats, &args.interface);
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
    
    info!("監視を終了しました");
    Ok(())
}