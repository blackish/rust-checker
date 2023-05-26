extern crate pnet;
pub mod config;
pub mod checker;
pub mod process;
pub mod output;
pub mod output_print;
pub mod selector;
pub mod stats_process;
pub mod pinger;

use crate::config::{load_config};
use crate::checker::CheckResult;
use crate::pinger::{IcmpChecker, icmp_sender, icmp_receiver};
use crate::process::{Processes, process_worker};
use crate::selector::selector_worker;
use crate::stats_process::Stats;
use crate::output::output_worker;
use crate::output_print::PrintOutput;

use std::sync::{mpsc, Arc};
use std::thread;

fn main() {
    let mut cfg = load_config();
    let mut pinger_handles = Vec::<thread::JoinHandle<()>>::new();
    let (mut selector_tx, mut selector_rx) = mpsc::channel();
    let mut processes = Vec::new();
    for mut p in cfg.1 {
        if p.process_name == "stats" {
            let (mut process_tx, mut process_rx) = mpsc::channel();
            let processor = Stats::new(p.values.clone(), p.match_value.clone(), 5, p.labels_to_add.clone(), p.id.clone());
            let to_selector = selector_tx.clone();
            let rcv = thread::spawn(move || { process_worker(processor, to_selector, process_rx) });
            p.sender = Some(process_tx);
            pinger_handles.push(rcv);
            processes.push(p);
        }
    }
    let mut outputs = Vec::new();
    for mut o in cfg.2 {
        if o.output_name == "print" {
            let output = PrintOutput::new(o.id.clone());
            let (mut output_tx, mut output_rx) = mpsc::channel();
            let rcv = thread::spawn(move || { output_worker(output, output_rx) });
            o.sender = Some(output_tx);
            outputs.push(o);
        }
    }
    let rcv = thread::spawn(move || { selector_worker(selector_rx, processes, outputs) });
    pinger_handles.push(rcv);
    let hosts = cfg.0;
    for c in hosts {
        if c.check_type == "icmp" {
            let checker = Arc::new(IcmpChecker::new(&c));
            let sender = Arc::clone(&checker);
            let mut sender_tx = selector_tx.clone();
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
