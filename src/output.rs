use std::sync::mpsc::Receiver;
use crate::checker::CheckResult;

pub trait Outputs {
    fn process_probe(&mut self, probe: CheckResult);
}

pub fn output_worker<T: Outputs>(mut output: T, receiver: Receiver<CheckResult>) {
    for new_probe in receiver {
        output.process_probe(new_probe);
    }
}
