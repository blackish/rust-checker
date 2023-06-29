extern crate pnet;

use crate::config::ProbeConfig;
use std::sync::{Arc, Mutex};
use crate::checker::CheckResult;

use std::time::{Duration, Instant};
use std::sync::mpsc::Sender;
use rand::random;
use pnet::packet::ipv4;
use std::net::{IpAddr, ToSocketAddrs};
use std::thread;
use crate::pnet::packet::Packet;
use pnet::transport::{transport_channel, tcp_packet_iter};
use pnet::transport::TransportChannelType::{Layer3, Layer4};
use pnet::transport::TransportProtocol::{Ipv4};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::tcp::{MutableTcpPacket,TcpOption, ipv4_checksum};
use pnet::util::checksum;
use std::collections::HashMap;

pub struct SynChecker {
    host: String,
    port: u16,
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
            name: config.name.clone(),
            host: config.host.clone(),
            interval: config.interval.clone(),
            source_ip: config.config.get("source_ip").unwrap().clone().into_string().unwrap(),
            probes: Mutex::new(Vec::<Probe>::new()),
            port: config.config.get("port").unwrap().clone().into_i64().unwrap() as u16,
            labels: config.labels.clone()
        }
    }
}

pub fn syn_sender(checker: &Arc<SynChecker>) {
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
        tcp.set_destination(checker.port);
        tcp.set_sequence(seq);
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
        ip.set_destination(addr);
        ip.set_source(saddr);
        ip.set_checksum(checksum(ip.packet(), 1));
        match tx.send_to(ip, IpAddr::V4(addr)) {
            Ok(_) => {
                checker.probes.lock().unwrap().push(Probe{seq: seq+1, sent: Instant::now()});
            },
            Err(e) => {
                println!("Error sending {:?}", e)
            }
        }
        thread::sleep(Duration::from_secs(checker.interval as u64));
    }
}

pub fn syn_receiver(checker: &Arc<SynChecker>, sender: Sender<CheckResult>) {
    let (_, mut rx) = transport_channel(4096, Layer4(Ipv4(IpNextHeaderProtocols::Tcp))).unwrap();
    let mut host = checker.host.clone();
    let timeout = Duration::new(1, 0);
    host.push_str(":0");
    let addr = host.to_socket_addrs()
        .unwrap()
        .next()
        .unwrap()
        .ip();
    let mut iter = tcp_packet_iter(&mut rx);
    loop {
        match iter.next_with_timeout(timeout) {
            Ok(result) => match result {
                Some((packet, raddr)) => {
                    if raddr == addr && packet.get_source() == checker.port && packet.get_destination() == 6535 {
                        let now = Instant::now();
                        let mut probes = checker.probes.lock().unwrap();
                        for probe in 0..probes.len() {
                            if probes[probe].seq == packet.get_acknowledgement() {
                                let finished_probe = probes.swap_remove(probe);
                                let mut to_emit = CheckResult{
                                    name: checker.name.clone(),
                                    values: HashMap::new(),
                                    processes: Vec::new(),
                                    labels: checker.labels.clone()};
                                to_emit.values.insert(String::from("rtt"), now.duration_since(finished_probe.sent).as_millis() as f32);
                                sender.send(to_emit).unwrap();
                                let mut to_emit = CheckResult{
                                    name: checker.name.clone(),
                                    values: HashMap::new(),
                                    processes: Vec::new(),
                                    labels: checker.labels.clone()};
                                to_emit.values.insert(String::from("loss"), 0.0);
                                sender.send(to_emit).unwrap();
                                break;
                            }
                        }
                    }
                },
                None => {
                    println!("Error decoding tcp packet");
                }
            },
            Err(_) => {
                panic!("Error getting packet");
            }
        }
        {
            let now = Instant::now();
            let mut probes = checker.probes.lock().unwrap();
            for probe in probes.len()..0 {
                if now.duration_since(probes[probe-1].sent) > timeout {
                    probes.swap_remove(probe-1);
                    let mut to_emit = CheckResult{
                        name: checker.name.clone(),
                        values: HashMap::new(),
                        processes: Vec::new(),
                        labels: checker.labels.clone()};
                    to_emit.values.insert(String::from("loss"), 1.0);
                    sender.send(to_emit).unwrap();
                }
            }
        }
    }
}
