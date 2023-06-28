use std::collections::HashMap;
use crate::checker::CheckResult;
use crate::process::Processes;
use crate::config::ProcessConfig;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::thread::sleep;
use std::time::Duration;

pub struct Histogram {
    keep_name: bool,
    stats_to_emit: Vec<f64>,
    stats: Arc<Mutex<HashMap<String, Vec<EmitStats>>>>,
    probes: Arc<Mutex<HashMap<String, CheckResult>>>,
    to_process: String,
    interval: i32,
    labels_to_add: HashMap<String, String>,
    receiver: Receiver<CheckResult>,
    id: u16
}

fn emit_probes(interval: i32, stats: Arc<Mutex<HashMap<String, Vec<EmitStats>>>>, probes: Arc<Mutex<HashMap<String, CheckResult>>>, sender: Sender<CheckResult>) {
    let duration = Duration::from_secs(interval.try_into().unwrap());
    println!("{:?}", duration);
    loop {
        sleep(duration);
        let mut to_emit_stats = stats.lock().unwrap();
        let to_emit_probes = probes.lock().unwrap();
        for (key, mut value) in to_emit_stats.drain() {
            let mut n_probe: i32 = 1;
            let probe_to_copy = to_emit_probes.get(&key).unwrap().clone();
            let mut new_probe = CheckResult {
                name: probe_to_copy.name.clone(),
                labels: probe_to_copy.labels.clone(),
                processes: probe_to_copy.processes.clone(),
                values: HashMap::new()
            };
            if let EmitStats::Number(n) = value.pop().unwrap() {
                n_probe = n;
            }
            for atomic_stat in value {
                match atomic_stat {
                    EmitStats::Sum(s) => _ = new_probe.values.insert(String::from("sum"), s),
                    EmitStats::Avg(s) => _ = new_probe.values.insert(String::from("avg"), s / n_probe as f32),
                    EmitStats::Low(s) => _ = new_probe.values.insert(String::from("low"), s),
                    EmitStats::High(s) => _ = new_probe.values.insert(String::from("high"), s),
                    EmitStats::Number(_) => {}
                }
            }
            sender.send(new_probe).unwrap();
        }
    }
}

impl Histogram {
    pub fn new(config: &ProcessConfig, sender: Sender<CheckResult>, receiver: Receiver<CheckResult>) -> Self {
        let result = Self {
            keep_name: config.keep_name.clone(),
            stats_to_emit: Vec::new(),
            stats: Arc::new(Mutex::new(HashMap::new())),
            probes: Arc::new(Mutex::new(HashMap::new())),
            to_process: config.match_value.clone(),
            interval: config.config.get("interval").unwrap().clone().into_i64().unwrap() as i32,
            id: config.id.clone(),
            labels_to_add: config.labels_to_add.clone(),
            receiver: receiver
        };
        match config.config.get("values") {
            yaml_rust::Yaml::Array(ref h) => {
                for v in h {
                    result.stats_to_emit.push(v.clone().into_f64().unwrap());
                }
            },
            _ => {}
        }
        let stats = Arc::clone(&result.stats);
        let probes = Arc::clone(&result.probes);
        let interval = result.interval.clone();
        thread::spawn(move || { emit_probes(interval, stats, probes, sender) });
        return result
    }
}

impl Processes for Histogram {
    fn process_probe(&mut self) {
        loop {
            let probe = self.receiver.recv().unwrap();
            let mut processed_probes = 1;
            let mut new_result = Vec::new();
            let mut new_probe_to_emit = self.probes.lock().unwrap();
            if !new_probe_to_emit.contains_key(&probe.name) {
                    let mut new_probe = CheckResult {
                        name: probe.name.clone(),
                        values: HashMap::new(),
                        labels: probe.labels.clone(),
                        processes: probe.processes.clone()
                    };
                    new_probe.values = HashMap::new();
                    new_probe.labels.extend(self.labels_to_add.clone());
                    new_probe.processes.push(self.id.clone());
                    if self.keep_name {
                        new_probe.labels.insert(String::from("value"), self.to_process.clone());
                    }
                    new_probe_to_emit.insert(probe.name.clone(), new_probe);
            }
            let mut low: f64 = -99999;
            new_result.push(0.0);
            for s in &self.stats_to_emit {
                new_result.push(0.0);
            }
            new_result.push(0.0);

            let to_process = probe.values.get(&self.to_process).unwrap().clone();
            let mut new_stats = self.stats.lock().unwrap();
            new_stats.entry(probe.name.clone()).and_modify(|e| {
                for atomic_stat in e {
                    match atomic_stat {
                        EmitStats::Sum(s) => {
                            *s += to_process;
                        },
                        EmitStats::Avg(s) => {
                            *s += to_process;
                        },
                        EmitStats::Low(s) => {
                            if *s > to_process {
                                *s = to_process.clone();
                            }
                        },
                        EmitStats::High(s) => {
                            if *s < to_process {
                                *s = to_process.clone();
                            }
                        },
                        EmitStats::Number(s) => {
                            *s += 1;
                            processed_probes = s.clone();
                        }
                    }
                }
            }).or_insert(new_result);
        }
    }
}

