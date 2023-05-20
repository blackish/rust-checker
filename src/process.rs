use std::sync::mpsc::{Sender, Receiver};
use std::collections::HashMap;
use crate::checker::{CheckResult};

enum EmitStats {
    AVG(f64),
    LOW(f64),
    High(f64),
    Number(int)
}

pub struct Stats {
    stats_to_emit: Vec<String>,
    stats: HashMap<String, Vec<EmitStats>>,
    to_process: String,
    interval: int,
    labels_to_add: Vec<HashMap<String, String>>,
    to_emit: Vec<CheckResult>
}

trait Processes {
    fn process_probe(CheckResult);
}

impl Stats {
    fn new(stats: Vec<String>, metric_name: String, stats_interval: int, labels_to_add: Vec<HashMap<String, String>>) -> Self {
        Self {
            stats_to_emit: stats,
            stats: HashMap::new(),
            to_process: metric_name,
            interval: stats_interval,
            to_emit: Vec::new(),
            labels: labels_to_add
        }
    }
}

impl Iterator for Stats {
    fn next(self) -> Option<CheckResult> {
        return self.to_emit.pop();
    }
}

impl Processes for Stats {
    fn process_probe(self, probe: CheckResult) {
        let mut processed_probes = 1;
        let mut new_result = Vec::new();
        for s in self.stats_to_emit {
            if s == 'avg' {
                new_result.push(EmitStats::Avg(probe.entry(self.to_process).unwrap()));
            } else if s == 'low' {
                new_result.push(EmitStats::Low(probe.entry(self.to_process).unwrap()));
            } else if s == 'high' {
                new_result.push(EmitStats::High(probe.entry(self.to_process).unwrap()));
            }
        }
        new_result.push(EmitStats::Number(1));

        let to_process = probe.entry(self.to_process).unwrap();
        stats.entry(probe.name).and_modify(|e| {
            for atomic_stat in e {
                match atomic_stat {
                    Avg(s) => s += probe.entry(self.to_process).unwrap(),
                    Low(s) => {
                        if s > probe.entry(self.to_process).unwrap() {
                            s = probe.entry(self.to_process).unwrap();
                        }
                    },
                    High(s) => {
                        if s < probe.entry(self.to_process).unwrap() {
                            s = probe.entry(self.to_process).unwrap();
                        }
                    },
                    Number(s) => {
                        s += 1;
                        processed_probes = s.clone();
                    }
                }
            }
        }).or_insert(new_result);
        if processed_probes >= self.interval {
            let mut entry = self.stats.remove(probe.name).unwrap();
            let mut check_to_emit = CheckResult{
                    name: probe.name,
                    values: HashMap::new(),
                    labels: probe.labels};
            check_to_emit.labels.extend(&self.labels_to_add);
            for atomic_stat in entry {
                match atomic_stats {
                    Avg(s) => {
                        check_to_emit.values.insert("avg", (s / processed_probes as f64));
                    },
                    Low(s) => check_to_emit.values.insert("low", s),
                    High(s) => check_to_emit.values.insert("high", s),
                    _ => {}
                }
            }
            self.to_emit.push(check_to_emit);
        }
    }
}

pub fn process_worker<T: Processes + Iterator>(stats: T, mut sender Sender<CheckResult>, mut receiver Receiver<CheckResult>) {
    loop {
        let probe = receiver.recv();
        match probe {
            Err(_) => {
                panic!("Stats processor has received an error");
            },
            Ok(new_probe) => {
                stats.process_probe(probe);
                for to_emit in stats {
                    sender.send(to_emit).unwrap();
                }
            }
        }
    }
};
