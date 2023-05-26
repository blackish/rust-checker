use crate::checker::CheckResult;
use crate::output::Outputs;

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
        let current_probe = probe;
        println!("Labels: {:?}\nValues: {:?}", current_probe.labels, current_probe.values);
    }
}
