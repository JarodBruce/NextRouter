mod capture;
mod prometheus_server;
mod stats;

use anyhow::Result;
use capture::start_network_monitoring_system;
use clap::Parser;
use log::{error, info};
use tokio::signal;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Network interface to monitor (default: ens19)
    #[arg(short, long, default_value = "ens19")]
    interface: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("Starting network traffic monitor with Prometheus integration");
    info!("Interface: {}", args.interface);

    // ルート権限の確認
    if unsafe { libc::geteuid() } != 0 {
        return Err(anyhow::anyhow!(
            "This program requires root privileges to capture network traffic"
        ));
    }
    // 指定インターフェースのIPアドレスとサブネットマスクを表示
    let (ip_addr, netmask) = match pnet_datalink::interfaces()
        .into_iter()
        .find(|iface| iface.name == args.interface)
    {
        Some(interface) => {
            for ip in &interface.ips {
                info!(
                    "Interface {}: IP address = {}, netmask = {}",
                    args.interface,
                    ip.ip(),
                    ip.mask()
                );
            }
            // Use the first IP address for monitoring
            if let Some(ip) = interface.ips.first() {
                (ip.ip(), ip.mask())
            } else {
                return Err(anyhow::anyhow!(
                    "No IP addresses found for interface '{}'",
                    args.interface
                ));
            }
        }
        None => {
            return Err(anyhow::anyhow!("Interface '{}' not found", args.interface));
        }
    };

    // ネットワークモニタリングシステムを開始
    let interface_name = args.interface.clone();
    let monitoring_task = tokio::spawn(async move {
        let result = start_network_monitoring_system(
            &interface_name,
            Some(ip_addr),
            Some(match netmask {
                std::net::IpAddr::V4(v4) => v4,
                std::net::IpAddr::V6(_) => {
                    error!("IPv6 subnets not supported");
                    return;
                }
            }),
        )
        .await;

        if let Err(e) = result {
            error!("Network monitoring system failed: {}", e);
        }
    });

    // シグナルハンドリング（タイムアウト付き）
    let result = tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down...");
            Ok(())
        }
        result = monitoring_task => {
            match result {
                Ok(_) => {
                    info!("Monitoring task completed successfully");
                    Ok(())
                }
                Err(e) => {
                    error!("Monitoring task panicked: {}", e);
                    Err(anyhow::anyhow!("Task panicked: {}", e))
                }
            }
        }
        _ = tokio::time::sleep(tokio::time::Duration::from_secs(3600)) => {
            info!("Timeout reached, shutting down...");
            Ok(())
        }
    };

    // クリーンアップのための短い待機時間
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    info!("Network traffic monitor stopped");

    // 結果を確認して適切に終了
    match result {
        Ok(_) => {
            info!("Exiting gracefully");
            std::process::exit(0);
        }
        Err(e) => {
            error!("Exiting with error: {}", e);
            std::process::exit(1);
        }
    }
}
