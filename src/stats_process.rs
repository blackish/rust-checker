use std::sync::mpsc::{Sender, Receiver, channel};
use std::collections::HashMap;
use crate::checker::CheckResult;
use crate::config::ProcessConfig;
use crate::config::OutputConfig;
use crate::process::Processes;

enum EmitStats {
    Avg(f64),
    Low(f64),
    High(f64),
    Sum(f64),
    Number(i32)
}

pub struct Stats {
    stats_to_emit: Vec<String>,
    stats: HashMap<String, Vec<EmitStats>>,
    to_process: String,
    interval: i32,
    labels_to_add: HashMap<String, String>,
    id: u16,
    to_emit: Vec<CheckResult>
}

impl Stats {
    pub fn new(stats: Vec<String>, metric_name: String, stats_interval: i32, labels_to_add: HashMap<String, String>, id: u16) -> Self {
        Self {
            stats_to_emit: stats,
            stats: HashMap::new(),
            to_process: metric_name,
            interval: stats_interval,
            to_emit: Vec::new(),
            id: id,
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
            } else if s == "sum" {
                new_result.push(EmitStats::Sum(probe.values.get(&self.to_process).unwrap().clone()));
            }
        }
        new_result.push(EmitStats::Number(1));

        let mut to_process = probe.values.get(&self.to_process).unwrap().clone();
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
            let mut entry = self.stats.remove(&probe.name).unwrap();
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
                        _ = check_to_emit.values.insert(String::from("avg"), s / processed_probes as f64);
                    },
                    EmitStats::Low(s) => _ = check_to_emit.values.insert(String::from("low"), s),
                    EmitStats::High(s) => _ = check_to_emit.values.insert(String::from("high"), s),
                    EmitStats::Sum(s) => _ = check_to_emit.values.insert(String::from("sum"), s),
                    _ => {}
                }
            }
            self.to_emit.push(check_to_emit);
        }
    }
}
