extern crate pnet;
pub mod config;
pub mod checker;
pub mod process;

use crate::config::{load_config};
use crate::checker::{Checker, icmp_sender, icmp_receiver, CheckResult};
use crate::process::{Stats, process_worker};

use std::sync::{mpsc, Arc};
use std::thread;

fn main() {
    let cfg = load_config();
    let mut pinger_handles = Vec::<thread::JoinHandle<()>>::new();
    let (mut probe_tx, mut probe_rx) = mpsc::channel();
    let hosts = cfg.0;
    for c in hosts {
        if c.check_type == "icmp" {
            let checker = Arc::new(Checker::new(&c));
            let sender = Arc::clone(&checker);
            let mut sender_tx = probe_tx.clone();
            let rcv = thread::spawn(move || {icmp_receiver(&sender, sender_tx)});
            pinger_handles.push(rcv);
            let sender = Arc::clone(&checker);
            let rcv = thread::spawn(move || {icmp_sender(&sender)});
            pinger_handles.push(rcv);
        }
        println!("{:?} {:?} {:?}", &c.host, &c.check_type, &c.interval);
    }

    for handle in pinger_handles {
        handle.join();
    };
}
