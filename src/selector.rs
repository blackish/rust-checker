use regex::Regex;
use std::sync::mpsc::{Receiver, channel};
use crate::checker::CheckResult;
use crate::config::ProcessConfig;
use crate::config::OutputConfig;

pub fn selector_worker(receiver: Receiver<CheckResult>, processes: Vec<ProcessConfig>, outputs: Vec<OutputConfig>) {
    for probe in receiver {
        log::debug!("{:?} {:?}", probe.labels, probe.values);
        let mut gone_for_processing = false;
        let (mut to_send, _) = channel();
        'processes: for p in &processes {
            if probe.processes.contains(&p.id) {
                continue 'processes;
            }
            'probes: for (key, value) in probe.labels.clone() {
                match p.match_labels.get(&key) {
                    Some(v) => {
                        for regex_value in v {
                            let match_regex = Regex::new(regex_value).unwrap();
                            if match_regex.is_match(&value) {
                                continue 'probes;
                            }
                        }
                        continue 'processes;
                    },
                    None => {
                        continue 'processes;
                    }
                }
            }
            match probe.values.get(&p.match_value) {
                None => {
                    continue 'processes;
                },
                Some(_) => {}
            }
            to_send = p.sender.as_ref().unwrap().clone();
            gone_for_processing = true;
            break 'processes;
        }
        if gone_for_processing {
            to_send.send(probe).unwrap();
        } else {
            'outputs: for o in &outputs {
                'outputprobes: for (key, value) in probe.labels.clone() {
                    match o.match_labels.get(&key) {
                        Some(v) => {
                            for regex_value in v {
                                let match_regex = Regex::new(regex_value).unwrap();
                                if match_regex.is_match(&value) {
                                    continue 'outputprobes;
                                }
                            }
                            continue 'outputs;
                        },
                        None => {
                            continue 'outputs;
                        }
                    }
                }
                to_send = o.sender.as_ref().unwrap().clone();
                to_send.send(probe).unwrap();
                break 'outputs;
            }
        }
    }
}
