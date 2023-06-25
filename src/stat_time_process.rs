use yaml_rust::yaml;
use std::collections::HashMap;
use crate::checker::CheckResult;
use crate::process::Processes;
use std::sync::{Arc, Mutex};

enum EmitStats {
    Avg(f32),
    Low(f32),
    High(f32),
    Sum(f32),
    Number(i32)
}

pub struct Stats {
    keep_name: bool,
    stats_to_emit: Vec<String>,
    stats: Arc<Mutex<HashMap<String, Vec<EmitStats>>>>,
    probe_to_emit: Arc<Mutex<HashMap<String, CheckResult>>>,
    to_process: String,
    interval: i32,
    labels_to_add: HashMap<String, String>,
    id: u16,
    to_emit: Vec<CheckResult>
}

impl Stats {
    pub fn new(stats: Vec<String>, metric_name: String, keep_name: bool, labels_to_add: HashMap<String, String>, config: HashMap<String, yaml::Yaml>, id: u16) -> Self {
        Self {
            keep_name: keep_name,
            stats_to_emit: stats,
            stats: HashMap::new(),
            to_process: metric_name,
            interval: config.get("interval").unwrap().clone().into_i64().unwrap() as i32,
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
        let new_probe_to_emit = self.probe_to_emit.lock();
        if !new_probe_to_emit.contains_key(probe.name.clone()) {
                let new_probe = probe.clone();
                new_probe.values = HashMap::new();
                new_probe.labels.extend(self.labels_to_add.clone());
                new_probe.processes.push(self.id.clone());
                if self.keep_name {
                    new_probe.labels.insert(String::from("value"), self.to_process.clone());
                }
                new_probe_to_emit.insert(probe.name.clone(), new_probe);
        }
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
        let new_stats = self.stats.lock();
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
            if self.keep_name {
                check_to_emit.labels.insert(String::from("value"), self.to_process.clone());
            }
            self.to_emit.push(check_to_emit);
        }
    }
}

fn emit_probes(interval: i64, probes: Arc<Mutex<HashMap<String, Vec<EmitStats>>>>, checks: Arc<Mutex<HashMap<String, CheckResult>>>) {
    loop {
        sleep(interval);
        to_emit_probes = probes.lock();
        to_emit_checks = checks.lock();
        for (key, value) in to_emit_probes.drain() {
            let n_probe: i32 = 1;
            if let EmitStats::Number(n) == new_probe.pop() {
                n_probe = n;
            }
            new_probe = to_emit_checks.get(key).unwrap().clone();
            for atomic_stat in e {
                match atomic_stat {
                    EmitStats::Sum(s) => _ = new_probe.values.insert(String::from("sum"), s),
                    EmitStats::Avg(s) => _ = new_probe.values.insert(String::from("avg"), s / processed_probes as f32),
                    EmitStats::Low(s) => _ = check_to_emit.values.insert(String::from("low"), s),
                    EmitStats::High(s) => _ = check_to_emit.values.insert(String::from("high"), s),
        }

}
