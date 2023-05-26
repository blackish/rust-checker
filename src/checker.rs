use std::collections::HashMap;

pub struct CheckResult {
    pub name: String,
    pub values: HashMap<String, f64>,
    pub labels: HashMap<String, String>,
    pub processes: Vec<u16>
}
