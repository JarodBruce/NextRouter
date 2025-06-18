use anyhow::{Context, Result};
use log::{error, info, warn};
use pnet::datalink::{self, NetworkInterface};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::ipv6::Ipv6Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::udp::UdpPacket;
use pnet::packet::Packet;
use std::net::IpAddr;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

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
}

impl PacketCapture {
    /// 新しいPacketCaptureインスタンスを作成
    pub fn new(interface_name: &str, packet_sender: mpsc::Sender<PacketInfo>) -> Result<Self> {
        let interface = find_interface(interface_name)
            .context(format!("Failed to find interface: {}", interface_name))?;

        Ok(Self {
            interface,
            packet_sender,
        })
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
) -> Result<thread::JoinHandle<()>> {
    let capture = PacketCapture::new(interface_name, packet_sender)?;
    let interface_name = interface_name.to_string();

    let handle = thread::spawn(move || {
        info!("Starting background packet capture for interface: {}", interface_name);
        
        if let Err(e) = capture.start_capture() {
            error!("Packet capture failed for interface {}: {}", interface_name, e);
        }
        
        info!("Packet capture stopped for interface: {}", interface_name);
    });

    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_interfaces() {
        let interfaces = list_interfaces();
        assert!(!interfaces.is_empty(), "No network interfaces found");
        
        for iface in interfaces {
            println!("Interface: {} - {:?}", iface.name, iface.description);
        }
    }

    #[test]
    fn test_packet_info_creation() {
        let packet_info = PacketInfo {
            protocol: "TCP".to_string(),
            size: 1500,
            src_ip: Some("192.168.1.1".parse().unwrap()),
            dst_ip: Some("192.168.1.2".parse().unwrap()),
            src_port: Some(80),
            dst_port: Some(12345),
            timestamp: chrono::Utc::now(),
        };

        assert_eq!(packet_info.protocol, "TCP");
        assert_eq!(packet_info.size, 1500);
        assert!(packet_info.src_ip.is_some());
        assert!(packet_info.dst_ip.is_some());
        assert_eq!(packet_info.src_port, Some(80));
        assert_eq!(packet_info.dst_port, Some(12345));
    }
}
