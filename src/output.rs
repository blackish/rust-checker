use std::sync::mpsc::{Sender, Receiver, channel};
use std::collections::HashMap;
use crate::checker::CheckResult;

pub trait Outputs {
    fn process_probe(&mut self, probe: CheckResult);
}

pub struct PrintOutput {
    pub id: u16
}

impl PrintOutput {
    pub fn new(id: u16) -> Self {
        return Self{id: id}
    }
}

impl Outputs for PrintOutput {
    fn process_probe(&mut self, probe: CheckResult) {
        let mut current_probe = probe;
        println!("Labels: {:?}\nValues: {:?}", current_probe.labels, current_probe.values);
    }
}

pub fn output_worker<T: Outputs>(mut output: T, mut receiver: Receiver<CheckResult>) {
    loop {
        let probe = receiver.recv();
        match probe {
            Err(_) => {
                panic!("Stats processor has received an error");
            },
            Ok(new_probe) => {
                output.process_probe(new_probe);
            }
        }
    }
}
