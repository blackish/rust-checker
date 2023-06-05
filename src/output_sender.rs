use crate::checker::CheckResult;
use crate::output::Outputs;
use crate::config::OutputConfig;

use remote_checker::remote_probe_client::{RemoteProbeClient};
use remote_checker::{ProbeRequest};
pub mod remote_checker {
    tonic::include_proto!("rust_checker");
}

pub struct RemoteOutput {
    address: String
}

impl RemoteOutput {
    pub fn new(config: &OutputConfig) -> Self {
        return Self{address: config.config.get("address").unwrap().to_string()}
    }
}

impl Outputs for RemoteOutput {
#[tokio::main]
    async fn process_probe(&mut self, probe: CheckResult) {
        let current_probe = probe;
        let mut client = RemoteProbeClient::connect(format!("http://{}:50051", self.address)).await.unwrap();
        let request = tonic::Request::new(ProbeRequest {
            name: current_probe.name,
            labels: current_probe.labels,
            values: current_probe.values
        });
        client.get_probe(request).await.unwrap();
    }
}
