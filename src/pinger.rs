extern crate pnet;

use log::debug;
use crate::config::ProbeConfig;
use std::sync::{Arc, Mutex};
use std::net::{IpAddr, ToSocketAddrs};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::mpsc::Sender;
use rand::random;
use pnet::transport::{transport_channel, icmp_packet_iter};
use pnet::transport::TransportChannelType::{Layer3, Layer4};
use pnet::transport::TransportProtocol::Ipv4;
use pnet::packet::{ipv4, Packet};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::icmp::{IcmpType, IcmpTypes, echo_request};
use pnet::packet::icmp::echo_reply::EchoReplyPacket;
use pnet::util::checksum;
use std::collections::HashMap;
use crate::checker::CheckResult;
use yaml_rust::Yaml;

pub struct IcmpChecker {
    host: String,
    mtu: i64,
    interval: i64,
    source_ip: String,
    name: String,
    probes: Mutex<Vec<Probe>>,
    precision: i64,
    labels: HashMap<String, String>
}

struct Probe {
    identifier: u16,
    seq: u16,
    sent: Instant
}

impl IcmpChecker {
    pub fn new(config: &ProbeConfig) -> Self {
        Self{
            name: config.name.clone(),
            host: config.host.clone(),
            mtu: config.config.get("mtu").unwrap().clone().into_i64().unwrap(),
            interval: config.interval.clone(),
            source_ip: config.config.get("source_ip").unwrap().clone().into_string().unwrap(),
            precision: config.config.get("precision")
                .unwrap_or(&Yaml::Integer(1))
                .clone()
                .into_i64()
                .unwrap(),
            probes: Mutex::new(Vec::<Probe>::new()),
            labels: config.labels.clone()
        }
    }
}

pub fn icmp_sender(checker: &Arc<IcmpChecker>) {
    let (mut icmpv4_tx, _) = transport_channel(4096, Layer3(IpNextHeaderProtocols::Icmp)).unwrap();
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
    let minimum_icmp_size = echo_request::MutableEchoRequestPacket::minimum_packet_size();
    let minimum_ip_size = ipv4::MutableIpv4Packet::minimum_packet_size() + minimum_icmp_size;
    let payload = vec![0;checker.mtu as usize];
    loop {
        let mut ip_packet = vec![0; minimum_ip_size + payload.len()];
        let mut icmp_packet = vec![0; minimum_icmp_size + payload.len()];
        let id = random::<u16>();
        let seq: u16 = 0;
        let mut icmp = echo_request::MutableEchoRequestPacket::new(&mut icmp_packet[..]).unwrap();
        icmp.set_identifier(id);
        icmp.set_sequence_number(seq);
        icmp.set_icmp_type(IcmpTypes::EchoRequest);
        icmp.set_payload(&payload[..]);
        icmp.set_checksum(checksum(&icmp.packet(), 1));
        let mut ip = ipv4::MutableIpv4Packet::new(&mut ip_packet[..]).unwrap();
        ip.set_next_level_protocol(IpNextHeaderProtocols::Icmp);
        ip.set_ttl(255);
        ip.set_total_length((minimum_ip_size + payload.len()) as u16);
        ip.set_header_length(5);
        ip.set_version(4);
        ip.set_flags(2);
        ip.set_payload(&mut icmp_packet[..]);
        ip.set_destination(addr);
        ip.set_source(saddr);
        ip.set_checksum(checksum(&ip.packet(), 1));
        match icmpv4_tx.send_to(ip, IpAddr::V4(addr)) {
            Ok(_) => {
                checker.probes.lock().unwrap().push(Probe{identifier: id, seq: seq, sent: Instant::now()});
            },
            Err(e) => {
                debug!("Error sending {:?}", e);
            }
        }
        thread::sleep(Duration::from_secs(checker.interval as u64));
    }
}

pub fn icmp_receiver(checker: &Arc<IcmpChecker>, sender: Sender<CheckResult>) {
    let (_, mut icmpv4_rx) = transport_channel(4096, Layer4(Ipv4(IpNextHeaderProtocols::Icmp))).unwrap();
    let mut host = checker.host.clone();
    let timeout = Duration::new(1, 0);
    host.push_str(":0");
    let addr = host.to_socket_addrs()
        .unwrap()
        .next()
        .unwrap()
        .ip();
    let mut iter = icmp_packet_iter(&mut icmpv4_rx);
    loop {
        match iter.next_with_timeout(timeout) {
            Ok(result) => match result {
                Some((packet, raddr)) => match EchoReplyPacket::new(packet.packet()) {
                    Some(echo_reply) => {
                        if packet.get_icmp_type() == IcmpType::new(0) && raddr == addr {
                            let now = Instant::now();
                            let mut probes = checker.probes.lock().unwrap();
                            for probe in 0..probes.len() {
                                if probes[probe].identifier == echo_reply.get_identifier() && probes[probe].seq == echo_reply.get_sequence_number() {
                                    let finished_probe = probes.swap_remove(probe);
                                    let mut to_emit = CheckResult{
                                        name: checker.name.clone(),
                                        values: HashMap::new(),
                                        processes: Vec::new(),
                                        labels: checker.labels.clone()};
                                    to_emit.values.insert(
                                        String::from("rtt"),
                                        (now.duration_since(finished_probe.sent).as_micros() as f32) / checker.precision as f32);
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
                            debug!("Error getting packet");
                    }
                },
                None => {
                }
            },
            Err(_) => {
                debug!("Error getting packet");
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
