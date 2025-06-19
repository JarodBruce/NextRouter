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

impl ProtocolStats {
    /// 新しいProtocolStatsインスタンスを作成
    pub fn new() -> Self {
        Self::default()
    }

    /// パケットとバイト数を追加
    pub fn add_packet(&mut self, bytes: u64) {
        self.packet_count += 1;
        self.byte_count += bytes;
    }

    /// 指定された期間でレート計算を更新
    pub fn update_rates(&mut self, duration_secs: f64) {
        if duration_secs > 0.0 {
            self.packets_per_second = self.packet_count as f64 / duration_secs;
            self.bytes_per_second = self.byte_count as f64 / duration_secs;
        }
    }

    /// バイト数を人間が読みやすい形式に変換
    pub fn format_bytes(&self) -> String {
        format_bytes(self.byte_count)
    }

    /// 転送レートを人間が読みやすい形式に変換
    pub fn format_rate(&self) -> String {
        format_bytes(self.bytes_per_second as u64) + "/s"
    }
}

/// バイト数を人間が読みやすい形式に変換する関数
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: f64 = 1024.0;

    if bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= THRESHOLD && unit_index < UNITS.len() - 1 {
        size /= THRESHOLD;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

impl TrafficStatistics {
    /// 新しいTrafficStatisticsインスタンスを作成
    pub fn new(interface: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            start_time: now,
            end_time: now,
            interface,
            total: ProtocolStats::new(),
            protocols: HashMap::new(),
            source_ips: HashMap::new(),
            destination_ips: HashMap::new(),
            ports: HashMap::new(),
            previous_total: ProtocolStats::new(),
            last_reset_time: now,
        }
    }

    /// 統計情報をリセット
    pub fn reset(&mut self) {
        let now = chrono::Utc::now();
        self.start_time = now;
        self.end_time = now;
        self.total = ProtocolStats::new();
        self.protocols.clear();
        self.source_ips.clear();
        self.destination_ips.clear();
        self.ports.clear();
        self.previous_total = ProtocolStats::new();
        self.last_reset_time = now;
    }

    /// パケット情報を記録
    pub fn record_packet(
        &mut self,
        protocol: &str,
        packet_size: u64,
        src_ip: Option<IpAddr>,
        dst_ip: Option<IpAddr>,
        src_port: Option<u16>,
        dst_port: Option<u16>,
    ) {
        // 合計統計を更新
        self.total.add_packet(packet_size);

        // プロトコル別統計を更新
        self.protocols
            .entry(protocol.to_string())
            .or_insert_with(ProtocolStats::new)
            .add_packet(packet_size);

        // IP統計を更新
        if let Some(ip) = src_ip {
            *self.source_ips.entry(ip).or_insert(0) += packet_size;
        }
        if let Some(ip) = dst_ip {
            *self.destination_ips.entry(ip).or_insert(0) += packet_size;
        }

        // ポート統計を更新
        if let Some(port) = src_port {
            *self.ports.entry(port).or_insert(0) += 1;
        }
        if let Some(port) = dst_port {
            *self.ports.entry(port).or_insert(0) += 1;
        }

        self.end_time = chrono::Utc::now();
    }

    /// 統計期間の長さ（秒）を取得
    pub fn duration_seconds(&self) -> f64 {
        let duration = self.end_time - self.start_time;
        duration.num_milliseconds() as f64 / 1000.0
    }

    /// すべての統計のレートを更新
    pub fn update_all_rates(&mut self) {
        let duration = self.duration_seconds();
        
        self.total.update_rates(duration);
        
        for stats in self.protocols.values_mut() {
            stats.update_rates(duration);
        }
    }

    /// 上位Nの送信元IPを取得
    pub fn top_source_ips(&self, n: usize) -> Vec<(IpAddr, u64)> {
        let mut sorted: Vec<_> = self.source_ips.iter().map(|(&ip, &bytes)| (ip, bytes)).collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.into_iter().take(n).collect()
    }

    /// 上位Nの宛先IPを取得
    pub fn top_destination_ips(&self, n: usize) -> Vec<(IpAddr, u64)> {
        let mut sorted: Vec<_> = self.destination_ips.iter().map(|(&ip, &bytes)| (ip, bytes)).collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.into_iter().take(n).collect()
    }

    /// 上位Nのポートを取得
    pub fn top_ports(&self, n: usize) -> Vec<(u16, u64)> {
        let mut sorted: Vec<_> = self.ports.iter().map(|(&port, &count)| (port, count)).collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.into_iter().take(n).collect()
    }

    /// 前回からの差分統計を計算して返す
    pub fn get_interval_stats(&mut self) -> ProtocolStats {
        let current_time = chrono::Utc::now();
        let interval_duration = (current_time - self.last_reset_time).num_milliseconds() as f64 / 1000.0;
        
        // 差分を計算
        let mut interval_stats = ProtocolStats {
            packet_count: self.total.packet_count - self.previous_total.packet_count,
            byte_count: self.total.byte_count - self.previous_total.byte_count,
            packets_per_second: 0.0,
            bytes_per_second: 0.0,
        };
        
        // レートを計算
        if interval_duration > 0.0 {
            interval_stats.packets_per_second = interval_stats.packet_count as f64 / interval_duration;
            interval_stats.bytes_per_second = interval_stats.byte_count as f64 / interval_duration;
        }
        
        // 前回の統計を更新
        self.previous_total = self.total.clone();
        self.last_reset_time = current_time;
        
        interval_stats
    }

    /// 統計情報の要約を文字列で取得（1秒間の差分）
    pub fn summary(&mut self) -> String {
        let interval_stats = self.get_interval_stats();
        
        format!(
            "Last Second | Rate: {:.1} packets/s, {}",
            interval_stats.packets_per_second,
            interval_stats.format_rate()
        )
    }
}

#[derive(Debug, Default, Clone)]
pub struct IpStats {
    pub tx_bytes: u64,
    pub rx_bytes: u64,
    pub tx_packets: u64,
    pub rx_packets: u64,
    pub tx_bytes_per_sec: u64,
    pub rx_bytes_per_sec: u64,
    pub tx_packets_per_sec: u64,
    pub rx_packets_per_sec: u64,
}

pub type IpStatsMap = Arc<Mutex<HashMap<IpAddr, IpStats>>>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1), "1 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_protocol_stats() {
        let mut stats = ProtocolStats::new();
        assert_eq!(stats.packet_count, 0);
        assert_eq!(stats.byte_count, 0);

        stats.add_packet(100);
        assert_eq!(stats.packet_count, 1);
        assert_eq!(stats.byte_count, 100);

        stats.add_packet(200);
        assert_eq!(stats.packet_count, 2);
        assert_eq!(stats.byte_count, 300);

        stats.update_rates(1.0);
        assert_eq!(stats.packets_per_second, 2.0);
        assert_eq!(stats.bytes_per_second, 300.0);
    }

    #[test]
    fn test_traffic_statistics() {
        let mut stats = TrafficStatistics::new("test0".to_string());
        let ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));

        stats.record_packet("TCP", 100, Some(ip1), Some(ip2), Some(80), Some(12345));
        stats.record_packet("UDP", 50, Some(ip2), Some(ip1), Some(53), Some(54321));

        assert_eq!(stats.total.packet_count, 2);
        assert_eq!(stats.total.byte_count, 150);
        assert_eq!(stats.protocols.len(), 2);
        assert_eq!(stats.source_ips.len(), 2);
        assert_eq!(stats.destination_ips.len(), 2);
        assert_eq!(stats.ports.len(), 4);

        let top_ips = stats.top_source_ips(10);
        assert!(top_ips.len() <= 2);
    }
}
