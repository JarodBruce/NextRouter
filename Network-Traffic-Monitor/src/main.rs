mod capture;
mod stats;
mod prometheus_server;

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

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Port for metrics server (default: 9090)
    #[arg(short = 'm', long, default_value = "9090")]
    metrics_port: u16,

    /// Prometheus server URL for pushing metrics (optional)
    #[arg(short = 'p', long)]
    prometheus_url: Option<String>,

    /// Interval in seconds for sending metrics to Prometheus (default: 15)
    #[arg(short = 't', long, default_value = "15")]
    prometheus_interval: u64,
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

    info!("Starting network traffic monitor with Prometheus integration");
    info!("Interface: {}", args.interface);
    info!("Metrics server port: {}", args.metrics_port);
    if let Some(ref url) = args.prometheus_url {
        info!("Prometheus URL: {}", url);
        info!("Prometheus push interval: {} seconds", args.prometheus_interval);
    }

    // ルート権限の確認
    if unsafe { libc::geteuid() } != 0 {
        return Err(anyhow::anyhow!(
            "This program requires root privileges to capture network traffic"
        ));
    }

    // ネットワークモニタリングシステムを開始
    let monitoring_task = tokio::spawn(async move {
        if let Err(e) = start_network_monitoring_system(
            &args.interface,
            args.metrics_port,
            args.prometheus_url.as_deref(),
            args.prometheus_interval,
        ).await {
            error!("Network monitoring system failed: {}", e);
        }
    });

    // シグナルハンドリング
    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down...");
        }
        _ = monitoring_task => {
            error!("Monitoring task ended unexpectedly");
        }
    }

    info!("Network traffic monitor stopped");
    Ok(())
}
