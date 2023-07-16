use log::error;
use std::process;
use std::collections::HashMap;
use std::sync::mpsc::{Sender, Receiver};
use crate::checker::CheckResult;
use crate::process::Processes;
use crate::config::ProcessConfig;

enum EmitStats {
    Avg(f32),
    Low(f32),
    High(f32),
    Sum(f32),
    Number(i32)
}

pub struct StatsCount {
    keep_name: bool,
    stats_to_emit: Vec<String>,
    stats: HashMap<String, Vec<EmitStats>>,
    to_process: String,
    interval: i32,
    labels_to_add: HashMap<String, String>,
    id: u16,
    sender: Sender<CheckResult>,
    receiver: Receiver<CheckResult>
}

impl StatsCount {
    pub fn new(config: &ProcessConfig, sender: Sender<CheckResult>, receiver: Receiver<CheckResult>) -> Self {
        let mut result = Self {
            keep_name: config.keep_name.clone(),
            stats_to_emit: Vec::new(),
            stats: HashMap::new(),
            to_process: config.match_value.clone(),
            interval: config.config.get("interval").unwrap().clone().into_i64().unwrap() as i32,
            id: config.id.clone(),
            labels_to_add: config.labels_to_add.clone(),
            sender: sender,
            receiver: receiver
        };
        match config.config.get("values").unwrap() {
            yaml_rust::Yaml::Array(ref l) => {
                for v in l {
                    result.stats_to_emit.push(v.clone().into_string().unwrap());
                }
            },
            _ => {
                error!("values should be an array");
                process::exit(1);
            }
        };
        return result
    }
}

impl Processes for StatsCount {
    fn process_probe(&mut self) {
        loop {
            let probe = self.receiver.recv().unwrap();
            let mut processed_probes = 1;
            let mut new_result = Vec::new();
            for s in &self.stats_to_emit {
                if s == "avg" {
                    new_result.push(EmitStats::Avg(probe.values.get(&self.to_process).unwrap().clone()));
                } else if s == "low" {
                    new_result.push(EmitStats::Low(probe.values.get(&self.to_process).unwrap().clone()));
                } else if s == "high" {
                    new_result.push(EmitStats::High(probe.values.get(&self.to_process).unwrap().clone()));
                } else if s == "sum" {
                    new_result.push(EmitStats::Sum(probe.values.get(&self.to_process).unwrap().clone()));
                }
            }
            new_result.push(EmitStats::Number(1));

            let to_process = probe.values.get(&self.to_process).unwrap().clone();
            self.stats.entry(probe.name.clone()).and_modify(|e| {
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
            if processed_probes >= self.interval {
                let entry = self.stats.remove(&probe.name).unwrap();
                let mut check_to_emit = CheckResult{
                        name: probe.name,
                        values: HashMap::new(),
                        processes: probe.processes,
                        labels: probe.labels};
                check_to_emit.labels.extend(self.labels_to_add.clone());
                check_to_emit.processes.push(self.id.clone());
                for atomic_stat in entry {
                    match atomic_stat {
                        EmitStats::Avg(s) => {
                            _ = check_to_emit.values.insert(String::from("avg"), s / processed_probes as f32);
                        },
                        EmitStats::Low(s) => _ = check_to_emit.values.insert(String::from("low"), s),
                        EmitStats::High(s) => _ = check_to_emit.values.insert(String::from("high"), s),
                        EmitStats::Sum(s) => _ = check_to_emit.values.insert(String::from("sum"), s),
                        _ => {}
                    }
                }
                if self.keep_name {
                    check_to_emit.labels.insert(String::from("value"), self.to_process.clone());
                }
                self.sender.send(check_to_emit).unwrap();
            }
        }
    }
}
