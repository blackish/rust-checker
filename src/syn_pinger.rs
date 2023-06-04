extern crate pnet;

use crate::config::ProbeConfig;
use std::sync::{Arc, Mutex};
use crate::checker::CheckResult;

use std::net::IpAddr::V4;
use std::time::Duration;
use pnet::packet::icmp::echo_request;
use pnet::packet::ipv4;
use std::net::{IpAddr, Ipv4Addr};
use std::thread;
use crate::pnet::packet::Packet;
use pnet::packet::{icmp};
use pnet::transport::transport_channel;
use pnet::transport::TransportChannelType::Layer3;
use pnet::transport::TransportProtocol::{Ipv4};
use pnet::packet::icmp::IcmpTypes;
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::tcp::{TcpPacket,MutableTcpPacket,TcpOption, TcpFlags, ipv4_checksum};
use pnet::transport::{icmp_packet_iter, ipv4_packet_iter};
use pnet::util::checksum;

pub struct SynChecker {
    host: String,
    port: i64,
    interval: i64,
    source_ip: String,
    name: String,
    probes: Mutex<Vec<Probe>>,
    labels: HashMap<String, String>
}

struct Probe {
    seq: u32,
    sent: Instant
}

impl SynChecker {
    pub fn new(config: &ProbeConfig) -> Self {
        Self{
            name: config.labels.get(&String::from("name")).clone().unwrap().to_string(),
            host: config.host.clone(),
            mtu: config.config.get("port").unwrap().parse().unwrap(),
            interval: config.interval.clone(),
            source_ip: config.config.get("source_ip").unwrap().to_string(),
            probes: Mutex::new(Vec::<Probe>::new()),
            labels: config.labels.clone()
        }
    }
}

pub fn syn_sender(checker: &Arc<IcmpChecker>) {
    let mut host = checker.host.clone();
    host.push_str(":0");
    let mut source = checker.source_ip.clone();
    source.push_str(":0");
    let addr = match host.to_socket_addrs()
        .unwrap()
        .next()
        .unwrap()
        .ip() {
            IpAddr::V4(ip) => ip,
            IpAddr::V6(_) => {
                return;
            }
        };
    let saddr = match source.to_socket_addrs()
        .unwrap()
        .next()
        .unwrap()
        .ip() {
            IpAddr::V4(ip) => ip,
            IpAddr::V6(_) => {
                return;
            }
        };
    let (mut tx, _) = transport_channel(4096, Layer3(IpNextHeaderProtocols::Tcp)).unwrap();
    let payload = vec![0;1];
    let minimum_tcp_size = MutableTcpPacket::minimum_packet_size();
    let minimum_ip_size = ipv4::MutableIpv4Packet::minimum_packet_size() + minimum_tcp_size;
    loop {
        let mut ip_packet = vec![0; minimum_ip_size + 0 + 19];
        let mut tcp_packet = vec![0; minimum_tcp_size + 0 + 19];
        let mut tcp = MutableTcpPacket::new(&mut tcp_packet[..]).unwrap();
        let seq = random::<u32>();
        tcp.set_source(6535);
        tcp.set_destination(&checker.port);
        tcp.set_sequence(&seq);
        tcp.set_acknowledgement(0); // TCP header acknowledgement number
        tcp.set_data_offset(8); // TCP header data offset
        tcp.set_reserved(0); // TCP header reserved
        tcp.set_flags(2); // TCP header flags
        tcp.set_window(64240); // TCP header window size
        tcp.set_urgent_ptr(0); // TCP header urgent
        tcp.set_options(&vec![TcpOption::mss(1460), TcpOption::sack_perm(), TcpOption::nop(), TcpOption::wscale(8)]);
        tcp.set_checksum(ipv4_checksum(&tcp.to_immutable(), &saddr, &addr ));
        let mut ip = ipv4::MutableIpv4Packet::new(&mut ip_packet[..]).unwrap();
        ip.set_ttl(255);
        ip.set_total_length((minimum_ip_size + payload.len()) as u16 + 40);
        ip.set_next_level_protocol(IpNextHeaderProtocols::Tcp);
        ip.set_header_length(5);
        ip.set_version(4);
        ip.set_flags(2);
        ip.set_payload(&mut tcp_packet[..]);
        ip.set_destination(&addr);
        ip.set_source(&saddr);
        ip.set_checksum(checksum(ip.packet(), 1));
        match tx.send_to(ip, addr) {
            Ok(size) => {
                checker.probes.lock().unwrap().push(Probe{seq: seq, sent: Instant::now()});
            },
            Err(e) => {
                println!("Error sending {:?}", e)
            }
        }
        thread::sleep(Duration::from_secs(checker.interval as u64));
    }
}

pub fn syn_receiver(checker: &Arc<IcmpChecker>, sender: Sender<CheckResult>) {
    let (_, mut rx) = transport_channel(4096, Layer3(IpNextHeaderProtocols::Tcp)).unwrap();
    let mut host = checker.host.clone();
    let timeout = Duration::new(1, 0);
    host.push_str(":0");
    let addr = host.to_socket_addrs()
        .unwrap()
        .next()
        .unwrap()
        .ip();
    let mut iter = ip_packet_iter(&mut rx);
    loop {
    }
}

fn main() {
    let rcv = thread::spawn(move || {
            let mut iter = ipv4_packet_iter(&mut rx);
            loop {
                match iter.next() {
                    Ok((packet, addr)) => {
                        let tcp = TcpPacket::new(packet.payload());
                        println!("{:?} {:?}", addr, tcp);
                    },
                    Err(e) => {
                        println!("An error occurred while reading: {}", e);
                    }
                }
            }
        });
    let payload = vec![0;1];
    let minimum_tcp_size = MutableTcpPacket::minimum_packet_size();
    let minimum_ip_size = ipv4::MutableIpv4Packet::minimum_packet_size() + minimum_tcp_size;
    let mut ip_packet = vec![0; minimum_ip_size + 0 + 19];
    let mut tcp_packet = vec![0; minimum_tcp_size + 0 + 19];
    let mut tcp = MutableTcpPacket::new(&mut tcp_packet[..]).unwrap();
    tcp.set_source(6535);
    tcp.set_destination(2223);
    tcp.set_sequence(3414405313);
    tcp.set_acknowledgement(0); // TCP header acknowledgement number
    tcp.set_data_offset(8); // TCP header data offset
    tcp.set_reserved(0); // TCP header reserved
    tcp.set_flags(2); // TCP header flags
    tcp.set_window(64240); // TCP header window size
    tcp.set_urgent_ptr(0); // TCP header urgent
    tcp.set_options(&vec![TcpOption::mss(1460), TcpOption::sack_perm(), TcpOption::nop(), TcpOption::wscale(8)]);
    tcp.set_checksum(ipv4_checksum(&tcp.to_immutable(), &"92.223.65.188".parse::<Ipv4Addr>().unwrap(), &"213.108.129.5".parse::<Ipv4Addr>().unwrap() ));
    let addr = "213.108.129.5".parse::<IpAddr>();
    let mut ip = ipv4::MutableIpv4Packet::new(&mut ip_packet[..]).unwrap();
    ip.set_ttl(255);
    ip.set_total_length((minimum_ip_size + payload.len()) as u16 + 40);
    ip.set_next_level_protocol(IpNextHeaderProtocols::Tcp);
    ip.set_header_length(5);
    ip.set_version(4);
    ip.set_flags(2);
    println!("TCP {:?}", ip);
    ip.set_payload(&mut tcp_packet[..]);
    match addr {
        Ok(valid_addr) => {
            match valid_addr {
                V4(v4addr) => {
                    ip.set_destination(v4addr);
                    ip.set_source("92.223.65.188".parse::<Ipv4Addr>().unwrap());
                    let csum = checksum(ip.packet(), 1);
                    ip.set_checksum(csum);
                    println!("{:?}", ip);
                    match tx.send_to(ip, valid_addr) {
                        Ok(size) => {
                            println!("Sent {:?}", size);
                        },
                        Err(e) => {
                            println!("Error sending {:?}", e)
                        }
                    }
                },
                _ => {
                    println!("Not supported");
                }
            }
        },
        Err(e) => {
            println!("Invalid address {:?}", e);
        }
    };
    rcv.join();
}
