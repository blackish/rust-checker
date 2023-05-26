use std::sync::mpsc::{Sender, Receiver, channel};
use std::collections::HashMap;
use crate::checker::CheckResult;

pub trait Outputs {
    fn process_probe(&mut self, probe: CheckResult);
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
