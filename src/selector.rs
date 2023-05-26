use std::sync::mpsc::{Receiver, channel};
use crate::checker::CheckResult;
use crate::config::ProcessConfig;
use crate::config::OutputConfig;

pub fn selector_worker(receiver: Receiver<CheckResult>, processes: Vec<ProcessConfig>, outputs: Vec<OutputConfig>) {
    loop {
        let probe = receiver.recv().unwrap();
        let mut gone_for_processing = false;
        let (mut to_send, _) = channel();
        'processes: for p in &processes {
            if probe.processes.contains(&p.id) {
                continue 'processes;
            }
            for (key, value) in probe.labels.clone() {
                match p.match_labels.get(&key) {
                    Some(v) => {
                        if !v.contains(&value) {
                            continue 'processes;
                        }
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
                for (key, value) in probe.labels.clone() {
                    match o.match_labels.get(&key) {
                        Some(v) => {
                            if !v.contains(&value) {
                                continue 'outputs;
                            }
                        },
                        None => {
                            continue 'outputs;
                        }
                    }
                    to_send = o.sender.as_ref().unwrap().clone();
                    to_send.send(probe).unwrap();
                    break 'outputs;
                }
            }
        }
    }
}
