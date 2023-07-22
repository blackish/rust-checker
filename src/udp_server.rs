use crate::config::ProbeConfig;
use std::sync::mpsc::Sender;
use std::net::{UdpSocket};
use std::time::{Duration};
use crate::checker::CheckResult;
use std::collections::HashMap;

pub struct UdpServerChecker {
    host: String,
    name: String,
    labels: HashMap<String, String>
}

impl UdpServerChecker {
    pub fn new(config: &ProbeConfig) -> Self {
        Self{
            name: config.name.clone(),
            host: config.host.clone(),
            labels: config.labels.clone()
        }
    }
}

pub fn udp_server(checker: UdpServerChecker, sender: Sender<CheckResult>) {
    let socket = UdpSocket::bind(checker.host).unwrap();
    socket.set_write_timeout(Some(Duration::from_secs(1))).unwrap();
    let mut buffer = [0; 9600];
    loop {
        if let Ok((size, src_addr)) = socket.recv_from(&mut buffer) {
            socket.send_to(&buffer[..size], &src_addr).unwrap();
            let mut probe = CheckResult {
                name: checker.name.clone(),
                values: HashMap::new(),
                processes: Vec::new(),
                labels: checker.labels.clone()};
            probe.values.insert(src_addr.to_string(), 1.0);
            sender.send(probe).unwrap();
        }
    }
}
