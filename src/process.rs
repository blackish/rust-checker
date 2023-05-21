use std::sync::mpsc::{Sender, Receiver};
use std::collections::HashMap;
use crate::checker::{CheckResult};

enum EmitStats {
    Avg(f64),
    Low(f64),
    High(f64),
    Number(i32)
}

pub struct Stats {
    stats_to_emit: Vec<String>,
    stats: HashMap<String, Vec<EmitStats>>,
    to_process: String,
    interval: i32,
    labels_to_add: HashMap<String, String>,
    to_emit: Vec<CheckResult>
}

pub trait Processes {
    fn process_probe(&mut self, probe: CheckResult);
    fn get_to_emit(&mut self) -> Vec<CheckResult>;
}

impl Stats {
    fn new(stats: Vec<String>, metric_name: String, stats_interval: i32, labels_to_add: HashMap<String, String>) -> Self {
        Self {
            stats_to_emit: stats,
            stats: HashMap::new(),
            to_process: metric_name,
            interval: stats_interval,
            to_emit: Vec::new(),
            labels_to_add: labels_to_add
        }
    }
}

impl Processes for Stats {
    fn get_to_emit(&mut self) -> Vec<CheckResult> {
        return self.to_emit.drain(..).collect();
    }

    fn process_probe(&mut self, probe: CheckResult) {
        let mut processed_probes = 1;
        let mut new_result = Vec::new();
        for s in &self.stats_to_emit {
            if s == "avg" {
                new_result.push(EmitStats::Avg(probe.values.get(&self.to_process).unwrap().clone()));
            } else if s == "low" {
                new_result.push(EmitStats::Low(probe.values.get(&self.to_process).unwrap().clone()));
            } else if s == "high" {
                new_result.push(EmitStats::High(probe.values.get(&self.to_process).unwrap().clone()));
            }
        }
        new_result.push(EmitStats::Number(1));

        let mut to_process = probe.values.get(&self.to_process).unwrap().clone();
        self.stats.entry(probe.name.clone()).and_modify(|e| {
            for atomic_stat in e {
                match atomic_stat {
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
            let mut entry = self.stats.remove(&probe.name).unwrap();
            let mut check_to_emit = CheckResult{
                    name: probe.name,
                    values: HashMap::new(),
                    labels: probe.labels};
            check_to_emit.labels.extend(self.labels_to_add.clone());
            for atomic_stat in entry {
                match atomic_stat {
                    EmitStats::Avg(s) => {
                        _ = check_to_emit.values.insert(String::from("avg"), s / processed_probes as f64);
                    },
                    EmitStats::Low(s) => _ = check_to_emit.values.insert(String::from("low"), s),
                    EmitStats::High(s) => _ = check_to_emit.values.insert(String::from("high"), s),
                    _ => {}
                }
            }
            self.to_emit.push(check_to_emit);
        }
    }
}

pub fn process_worker<T: Processes>(mut stats: T, mut sender: Sender<CheckResult>, mut receiver: Receiver<CheckResult>) {
    loop {
        let probe = receiver.recv();
        match probe {
            Err(_) => {
                panic!("Stats processor has received an error");
            },
            Ok(new_probe) => {
                {
                    stats.process_probe(new_probe);
                }
                let mut to_emit = stats.get_to_emit();
                while to_emit.len() > 0 {
                    let to_send = to_emit.pop();
                    match to_send {
                        Some(c) => {
                            sender.send(c).unwrap();
                        },
                        None => {}
                    }
                }
            }
        }
    }
}
