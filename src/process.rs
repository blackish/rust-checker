use std::sync::mpsc::{Sender, Receiver};
use crate::checker::CheckResult;

pub trait Processes {
    fn process_probe(&mut self, probe: CheckResult);
    fn get_to_emit(&mut self) -> Vec<CheckResult>;
}

pub fn process_worker<T: Processes>(mut stats: T, sender: Sender<CheckResult>, receiver: Receiver<CheckResult>) {
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
