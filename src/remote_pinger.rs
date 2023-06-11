use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use tonic::{transport::Server, Request, Response, Status};
use futures::{StreamExt};
use crate::checker::CheckResult;

use remote_checker::remote_probe_server::{RemoteProbe, RemoteProbeServer};
use remote_checker::{ProbeReply, ProbeRequest};
pub mod remote_checker {
    tonic::include_proto!("rust_checker");
}

struct ProbeReceiver {
    sender: Arc<Mutex<Sender<CheckResult>>>
}

impl ProbeReceiver {
    fn new(to_sender: Sender<CheckResult>) -> Self {
        return Self {sender: Arc::new(Mutex::new(to_sender))}
    }
}

#[tonic::async_trait]
impl RemoteProbe for ProbeReceiver {
    async fn get_probe(
        &self,
        request: Request<tonic::Streaming<ProbeRequest>>,
    ) -> Result<Response<ProbeReply>, Status> {
        let mut stream = request.into_inner();
        let reply = ProbeReply {
            message: format!("Ok"),
            result: true,
        };
        println!("Starting listener");
        while let Some(probe) = stream.next().await {
            let probe = probe?;
            let new_probe = CheckResult{
                name: probe.name,
                labels: probe.labels,
                values: probe.values,
                processes: Vec::new()
            };
            println!("{:?}", new_probe);
            self.sender.lock().unwrap().send(new_probe).unwrap();
        }
        Ok(Response::new(reply))
    }
}

#[tokio::main]
pub async fn run_server(to_sender: Sender<CheckResult>) {
    let addr = "0.0.0.0:50051".parse().unwrap();
    let greeter = ProbeReceiver::new(to_sender);

    Server::builder()
        .add_service(RemoteProbeServer::new(greeter))
        .serve(addr)
        .await
        .unwrap();
}
