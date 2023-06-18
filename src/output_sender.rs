use crate::checker::CheckResult;
use crate::output::Outputs;
use crate::config::OutputConfig;
use std::thread;
use async_stream;
use tonic::Request;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use remote_checker::remote_probe_client::{RemoteProbeClient};
use remote_checker::{ProbeRequest};
pub mod remote_checker {
    tonic::include_proto!("rust_checker");
}

pub struct RemoteOutput {
    sender: mpsc::Sender<ProbeRequest>,
    rt: Runtime
}

impl RemoteOutput {
    pub fn new(config: &OutputConfig) -> Self {
        //let (tx, rx) = channel();
        let (tx, rx) = mpsc::channel(65535);
        let address = config.config.get("address").unwrap().to_string();
        thread::spawn(move || { stream_probe(address, rx)});
        let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
        return Self{sender: tx, rt: rt}
    }
}

#[tokio::main]
async fn stream_probe(address: String, mut rx: mpsc::Receiver<ProbeRequest>) {
    let mut client = RemoteProbeClient::connect(format!("http://{}:50051", address)).await.unwrap();
    let probe_stream = async_stream::stream! {
        while let Some(item) = rx.recv().await {
            yield item;
        }
    };
    let request = Request::new(probe_stream);
    match client.get_probe(request).await {
        Ok(response) => {},
        Err(_) => println!("Error")
    }
}

impl Outputs for RemoteOutput {
    fn process_probe(&mut self, probe: CheckResult) {
        let current_probe = probe;
        let request = ProbeRequest {
            name: current_probe.name,
            labels: current_probe.labels,
            values: current_probe.values
        };
        self.rt.block_on(self.sender.send(request)).unwrap();
    }
}
