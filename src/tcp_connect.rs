use crate::config::ProbeConfig;
use yaml_rust::Yaml;
use std::sync::mpsc::Sender;
use std::net::{TcpStream, Shutdown};
use std::net::ToSocketAddrs;
use std::thread;
use std::time::{Duration, Instant};
use crate::checker::CheckResult;
use std::collections::HashMap;

pub struct TcpConnectChecker {
    host: String,
    interval: i64,
    timeout: i64,
    name: String,
    precision: i64,
    labels: HashMap<String, String>
}

impl TcpConnectChecker {
    pub fn new(config: &ProbeConfig) -> Self {
        Self{
            name: config.name.clone(),
            host: config.host.clone(),
            interval: config.interval.clone(),
            timeout: config.config.get("timeout").unwrap().clone().into_i64().unwrap(),
            precision: config.config.get("precision")
                .unwrap_or(&Yaml::Integer(1))
                .clone()
                .into_i64()
                .unwrap(),
            labels: config.labels.clone()
        }
    }
}

pub fn tcp_connect(checker: TcpConnectChecker, sender: Sender<CheckResult>) {
    let addr = checker.host.to_socket_addrs().unwrap().next().unwrap();
    let timeout = Duration::from_secs(checker.timeout as u64);
    loop {
        let start = Instant::now();
        if let Ok(stream) = TcpStream::connect_timeout(&addr, timeout) {
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
            stream.shutdown(Shutdown::Both).unwrap();
        } else {
            let mut loss = CheckResult{
                name: checker.name.clone(),
                values: HashMap::new(),
                processes: Vec::new(),
                labels: checker.labels.clone()};
            loss.values.insert(String::from("loss"), 1.0);
            sender.send(loss).unwrap();
        }

        thread::sleep(Duration::from_secs(checker.interval as u64));
    }
}
