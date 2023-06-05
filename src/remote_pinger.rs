use std::sync::mpsc::Sender;
use std::sync::Mutex;
use tonic::{transport::Server, Request, Response, Status};
use crate::checker::CheckResult;

use remote_checker::remote_probe_server::{RemoteProbe, RemoteProbeServer};
use remote_checker::{ProbeReply, ProbeRequest};
pub mod remote_checker {
    tonic::include_proto!("rust_checker");
}

struct ProbeReceiver {
    sender: Mutex<Sender<CheckResult>>
}

impl ProbeReceiver {
    fn new(to_sender: Sender<CheckResult>) -> Self {
        return Self {sender: Mutex::new(to_sender)}
    }
}

#[tonic::async_trait]
impl RemoteProbe for ProbeReceiver {
    async fn get_probe(
        &self,
        request: Request<ProbeRequest>,
    ) -> Result<Response<ProbeReply>, Status> {
        let probe = request.into_inner();
        let new_probe = CheckResult{
            name: probe.name,
            labels: probe.labels,
            values: probe.values,
            processes: Vec::new()
        };
        println!("Got a request: {:?}", new_probe);
        let reply = ProbeReply {
            message: format!("Hello {}!", &new_probe.name),
            result: true,
        };
        self.sender.lock().unwrap().send(new_probe).unwrap();
        Ok(Response::new(reply))
    }
}

#[tokio::main]
pub async fn run_server(to_sender: Sender<CheckResult>) {
    let addr = "[::1]:50051".parse().unwrap();
    let greeter = ProbeReceiver::new(to_sender);

    Server::builder()
        .add_service(RemoteProbeServer::new(greeter))
        .serve(addr)
        .await
        .unwrap();
}
