use std::collections::HashMap;

#[derive(Debug)]
pub struct CheckResult {
    pub name: String,
    pub values: HashMap<String, f32>,
    pub labels: HashMap<String, String>,
    pub processes: Vec<u16>
}
