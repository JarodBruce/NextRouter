mod capture;
mod stats;

use anyhow::{Context, Result};
use capture::{start_capture_background, PacketInfo};
use clap::Parser;
use log::{error, info, warn};
use pnet::datalink;
use stats::{TrafficStatistics, format_bytes};
use std::net::IpAddr;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::signal;
use tokio::time::interval;

/// 指定されたインターフェースのIPアドレスを取得する関数
fn get_interface_ips(interface_name: &str) -> Vec<IpAddr> {
    let interfaces = datalink::interfaces();
    for interface in interfaces {
        if interface.name == interface_name {
            return interface.ips.iter().map(|ip| ip.ip()).collect();
        }
    }
    Vec::new()
}

/// ローカルIPアドレスまたは除外対象かどうかを判定する関数
fn is_local_or_excluded_ip(ip: &IpAddr, interface_name: &str) -> bool {
    // 指定されたインターフェースのIPアドレスをチェック
    let interface_ips = get_interface_ips(interface_name);
    if interface_ips.contains(ip) {
        return true; // インターフェース自身のIPアドレスは除外
    }
    
    match ip {
        IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            // プライベートIPアドレス範囲をチェック
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
            // ループバック 127.0.0.0/8
            if octets[0] == 127 {
                return true;
            }
            // リンクローカル 169.254.0.0/16
            if octets[0] == 169 && octets[1] == 254 {
                return true;
            }
            false
        }
        IpAddr::V6(ipv6) => {
            // IPv6のローカルアドレス判定
            ipv6.is_loopback() || 
            ipv6.is_unique_local() || 
            ipv6.is_unicast_link_local()
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Network interface to monitor (default: ens19)
    #[arg(short, long, default_value = "ens19")]
    interface: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

/// メイン統計管理構造体
struct TrafficMonitor {
    stats: Arc<Mutex<TrafficStatistics>>,
}

impl TrafficMonitor {
    fn new(interface: String) -> (Self, mpsc::Receiver<PacketInfo>) {
        let stats = Arc::new(Mutex::new(TrafficStatistics::new(interface)));
        let (_sender, receiver) = mpsc::channel();
        
        (Self {
            stats,
        }, receiver)
    }

    /// 統計をリセット
    fn reset_stats(&self) {
        let mut stats = self.stats.lock().unwrap();
        stats.reset();
    }

    /// 現在の統計情報のサマリーを取得
    fn get_summary(&self) -> String {
        let mut stats = self.stats.lock().unwrap();
        stats.summary()
    }

    /// 現在の統計情報を取得（表示用）
    fn get_current_stats_for_display(&self) -> TrafficStatistics {
        let mut stats = self.stats.lock().unwrap();
        stats.update_all_rates();
        stats.clone()
    }

    /// パケット情報を記録
    fn record_packet(&self, packet_info: PacketInfo) {
        let mut stats = self.stats.lock().unwrap();
        stats.record_packet(
            &packet_info.protocol,
            packet_info.size,
            packet_info.src_ip,
            packet_info.dst_ip,
            packet_info.src_port,
            packet_info.dst_port,
        );
    }
}

async fn packet_processing_task(
    monitor: Arc<TrafficMonitor>, 
    packet_receiver: mpsc::Receiver<PacketInfo>
) -> Result<()> {
    let mut _packet_count = 0u64;
    
    loop {
        match packet_receiver.try_recv() {
            Ok(packet_info) => {
                _packet_count += 1;
                monitor.record_packet(packet_info);
            }
            Err(mpsc::TryRecvError::Empty) => {
                // パケットがない場合は少し待機
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                warn!("Packet receiver disconnected");
                break;
            }
        }
    }
    
    Ok(())
}
async fn periodic_stats_output(monitor: Arc<TrafficMonitor>, interval_secs: u64) -> Result<()> {
    let mut interval_timer = interval(Duration::from_secs(interval_secs));

    loop {
        interval_timer.tick().await;
        
        let summary = monitor.get_summary();
        let stats = monitor.get_current_stats_for_display();
        
        info!("=== Traffic Statistics ===");
        info!("{}", summary);
        
        // プロトコル別詳細
        for (protocol, proto_stats) in &stats.protocols {
            if proto_stats.packet_count > 0 {
                info!("  {}: {} packets, {} ({}/s)", 
                    protocol, 
                    proto_stats.packet_count, 
                    proto_stats.format_bytes(),
                    proto_stats.format_rate()
                );
            }
        }

        // 上位送信元IP（ローカル・インターフェースIP除外）
        let top_src = stats.top_source_ips(10); // より多く取得してフィルタリング
        let local_src: Vec<_> = top_src.into_iter()
            .filter(|(ip, _)| is_local_or_excluded_ip(ip, &stats.interface))
            .take(5)
            .collect();
        if !local_src.is_empty() {
            info!("  Top Local Source IPs:");
            for (ip, bytes) in local_src {
                info!("    {} - {}", ip, format_bytes(bytes));
            }
        }

        // 上位宛先IP（ローカル・インターフェースIP除外）
        let top_dst = stats.top_destination_ips(10); // より多く取得してフィルタリング
        let local_dst: Vec<_> = top_dst.into_iter()
            .filter(|(ip, _)| is_local_or_excluded_ip(ip, &stats.interface))
            .take(5)
            .collect();
        if !local_dst.is_empty() {
            info!("  Top Local Destination IPs:");
            for (ip, bytes) in local_dst {
                info!("    {} - {}", ip, format_bytes(bytes));
            }
        }

        info!("========================");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
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

    info!("Starting network traffic monitor");
    info!("Interface: {}", args.interface);
    info!("Statistics interval: 1 seconds");

    // ルート権限の確認
    if unsafe { libc::geteuid() } != 0 {
        return Err(anyhow::anyhow!(
            "This program requires root privileges to capture network traffic"
        ));
    }

    // パケット通信用のチャネルを作成
    let (packet_sender, packet_receiver) = mpsc::channel::<PacketInfo>();

    // TrafficMonitorを作成
    let (traffic_monitor, _) = TrafficMonitor::new(
        args.interface.clone(),
    );

    let monitor_arc = Arc::new(traffic_monitor);

    // パケットキャプチャをバックグラウンドで開始
    let _capture_handle = start_capture_background(&args.interface, packet_sender)
        .context("Failed to start packet capture")?;

    // パケット処理タスクを開始
    let monitor_clone = Arc::clone(&monitor_arc);
    let packet_task = tokio::spawn(async move {
        if let Err(e) = packet_processing_task(monitor_clone, packet_receiver).await {
            error!("Packet processing failed: {}", e);
        }
    });

    // 統計出力タスクを開始（1秒間隔）
    let stats_clone = Arc::clone(&monitor_arc);
    let stats_task = tokio::spawn(async move {
        if let Err(e) = periodic_stats_output(stats_clone, 1).await {
            error!("Statistics output failed: {}", e);
        }
    });

    // シグナルハンドリング
    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down...");
        }
        _ = packet_task => {
            warn!("Packet processing task ended unexpectedly");
        }
        _ = stats_task => {
            warn!("Statistics task ended unexpectedly");
        }
    }

    // バックグラウンドタスクの終了を待機（タイムアウト付き）
    info!("Waiting for background tasks to finish...");
    
    // キャプチャスレッドは自動的に終了されるため、ここでは待機しない
    // capture_handle.join().unwrap();

    info!("Network traffic monitor stopped");
    Ok(())
}
