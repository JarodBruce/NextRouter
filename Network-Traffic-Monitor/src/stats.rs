use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};

/// ネットワークトラフィックの統計情報を格納する構造体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficStatistics {
    /// 統計の開始時刻
    pub start_time: chrono::DateTime<chrono::Utc>,
    /// 統計の終了時刻
    pub end_time: chrono::DateTime<chrono::Utc>,
    /// 監視対象インターフェース
    pub interface: String,
    /// 合計統計
    pub total: ProtocolStats,
    /// プロトコル別統計
    pub protocols: HashMap<String, ProtocolStats>,
    /// IP別統計（送信元）
    pub source_ips: HashMap<IpAddr, u64>,
    /// IP別統計（宛先）
    pub destination_ips: HashMap<IpAddr, u64>,
    /// ポート別統計
    pub ports: HashMap<u16, u64>,
    /// 前回の統計（差分計算用）
    pub previous_total: ProtocolStats,
    /// 最後のリセット時刻
    pub last_reset_time: chrono::DateTime<chrono::Utc>,
}

/// プロトコル別の統計情報
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProtocolStats {
    /// パケット数
    pub packet_count: u64,
    /// バイト数
    pub byte_count: u64,
    /// 秒あたりのパケット数
    pub packets_per_second: f64,
    /// 秒あたりのバイト数（Bps）
    pub bytes_per_second: f64,
}

#[derive(Debug, Default, Clone)]
pub struct IpStats {
    pub tx_bytes: u64,
    pub rx_bytes: u64,
}

pub type IpStatsMap = Arc<Mutex<HashMap<IpAddr, IpStats>>>;
