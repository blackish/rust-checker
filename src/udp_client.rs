use crate::config::ProbeConfig;
use yaml_rust::Yaml;
use std::sync::mpsc::Sender;
use std::net::UdpSocket;
use std::thread;
use std::time::{Duration, Instant};
use crate::checker::CheckResult;
use std::collections::HashMap;
use log::debug;

pub struct UdpClientChecker {
    host: String,
    interval: i64,
    timeout: i64,
    name: String,
    source: String,
    precision: i64,
    labels: HashMap<String, String>
}

impl UdpClientChecker {
    pub fn new(config: &ProbeConfig) -> Self {
        Self{
            name: config.name.clone(),
            host: config.host.clone(),
            interval: config.interval.clone(),
            timeout: config.config.get("timeout").unwrap().clone().into_i64().unwrap() as i64,
            source: config.config.get("source").unwrap().clone().into_string().unwrap(),
            precision: config.config.get("precision")
                .unwrap_or(&Yaml::Integer(1))
                .clone()
                .into_i64()
                .unwrap(),
            labels: config.labels.clone()
        }
    }
}

pub fn udp_client(checker: UdpClientChecker, sender: Sender<CheckResult>) {
    let socket = UdpSocket::bind(checker.source).unwrap();
    socket.set_write_timeout(Some(Duration::from_secs(checker.timeout as u64))).unwrap();
    socket.set_read_timeout(Some(Duration::from_secs(checker.timeout as u64))).unwrap();
    socket.connect(checker.host).unwrap();
    let interval = Duration::from_secs(checker.interval as u64);
    //let mut buffer: &[u8] = &[0; 9600];
    let mut buffer = [0; 9600];
    loop {
        let start = Instant::now();
        if let Ok(_) = socket.send(&[0; 1]) {
            if let Ok(_) = socket.recv(&mut buffer) {
                let mut rtt = CheckResult{
                    name: checker.name.clone(),
                    values: HashMap::new(),
                    processes: Vec::new(),
                    labels: checker.labels.clone()};
                rtt.values.insert(
                    String::from("rtt"),
                    (Instant::now().duration_since(start).as_micros() as f32) / checker.precision as f32);
                let mut loss = CheckResult{
                    name: checker.name.clone(),
                    values: HashMap::new(),
                    processes: Vec::new(),
                    labels: checker.labels.clone()};
                loss.values.insert(String::from("loss"), 0.0);
                sender.send(rtt).unwrap();
                sender.send(loss).unwrap();
            } else {
                let mut loss = CheckResult{
                    name: checker.name.clone(),
                    values: HashMap::new(),
                    processes: Vec::new(),
                    labels: checker.labels.clone()};
                loss.values.insert(String::from("loss"), 1.0);
                sender.send(loss).unwrap();
            }
        } else {
            debug!("Failed to send probe");
        }
        thread::sleep(interval);
    }
}
