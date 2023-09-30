extern crate pnet;
pub mod config;
pub mod checker;
pub mod process;
pub mod output;
pub mod output_print;
pub mod selector;
pub mod stats_process;
pub mod stats_time_process;
pub mod histogram_process;
pub mod pinger;
pub mod syn_pinger;
pub mod remote_pinger;
pub mod tcp_connect;
pub mod output_sender;
pub mod output_graphite;
pub mod mtu_pinger;
pub mod udp_server;
pub mod udp_client;

use log::{info, debug};
use crate::config::load_config;
use crate::remote_pinger::run_server;
use crate::pinger::{IcmpChecker, icmp_sender, icmp_receiver};
use crate::mtu_pinger::{IcmpMtuChecker, icmp_mtu_sender, icmp_mtu_receiver};
use crate::syn_pinger::{SynChecker, syn_sender, syn_receiver};
use crate::tcp_connect::{TcpConnectChecker, tcp_connect};
use crate::udp_server::{UdpServerChecker, udp_server};
use crate::udp_client::{UdpClientChecker, udp_client};
use crate::selector::selector_worker;
use crate::stats_process::StatsCount;
use crate::stats_time_process::StatsTime;
use crate::histogram_process::Histogram;
use crate::process::Processes;
use crate::output::output_worker;
use crate::output_print::PrintOutput;
use crate::output_sender::RemoteOutput;
use crate::output_graphite::GraphiteOutput;

use std::sync::{mpsc, Arc};
use std::thread;

fn main() {
    env_logger::init();
    let cfg = load_config();
    let mut pinger_handles = Vec::<thread::JoinHandle<()>>::new();
    let (selector_tx, selector_rx) = mpsc::channel();
    let mut processes = Vec::new();
    info!("Starting processes...");
    for mut new_process in cfg.1 {
        if new_process.process_name == "stats_count" {
            info!("Starting stats_count");
            let (process_tx, process_rx) = mpsc::channel();
            new_process.sender = Some(process_tx);
            let to_selector = selector_tx.clone();
            let mut processor = StatsCount::new(&new_process, to_selector, process_rx);
            let rcv = thread::spawn(move || { processor.process_probe() });
            pinger_handles.push(rcv);
            processes.push(new_process);
        } else if new_process.process_name == "stats_time" {
            info!("  Starting stats_time");
            let (process_tx, process_rx) = mpsc::channel();
            new_process.sender = Some(process_tx);
            let to_selector = selector_tx.clone();
            let mut processor = StatsTime::new(&new_process, to_selector, process_rx);
            let rcv = thread::spawn(move || { processor.process_probe() });
            pinger_handles.push(rcv);
            processes.push(new_process);
        } else if new_process.process_name == "histogram" {
            info!("  Starting histogram");
            let (process_tx, process_rx) = mpsc::channel();
            new_process.sender = Some(process_tx);
            let to_selector = selector_tx.clone();
            let mut processor = Histogram::new(&new_process, to_selector, process_rx);
            let rcv = thread::spawn(move || { processor.process_probe() });
            pinger_handles.push(rcv);
            processes.push(new_process);
        }
    }
    let mut outputs = Vec::new();
    info!("Starting outputs...");
    for mut new_output in cfg.2 {
        if new_output.output_name == "print" {
            info!("  Starting print output");
            let output = PrintOutput::new(new_output.id.clone());
            let (output_tx, output_rx) = mpsc::channel();
            let rcv = thread::spawn(move || { output_worker(output, output_rx) });
            pinger_handles.push(rcv);
            new_output.sender = Some(output_tx);
            outputs.push(new_output);
        } else if new_output.output_name == "remote_sender" {
            info!("  Starting remote_sender output");
            let output = RemoteOutput::new(&new_output);
            let (output_tx, output_rx) = mpsc::channel();
            let rcv = thread::spawn(move || { output_worker(output, output_rx) });
            pinger_handles.push(rcv);
            new_output.sender = Some(output_tx);
            outputs.push(new_output);
        } else if new_output.output_name == "graphite" {
            info!("  Starting graphite output");
            let output = GraphiteOutput::new(&new_output);
            let (output_tx, output_rx) = mpsc::channel();
            let rcv = thread::spawn(move || { output_worker(output, output_rx) });
            pinger_handles.push(rcv);
            new_output.sender = Some(output_tx);
            outputs.push(new_output);
        }
    }
    let rcv = thread::spawn(move || { selector_worker(selector_rx, processes, outputs) });
    pinger_handles.push(rcv);
    let hosts = cfg.0;
    info!("Starting probes");
    for new_check in hosts {
        if new_check.check_type == "icmp" {
            info!("  Starting icmp for {}", new_check.host);
            let checker = Arc::new(IcmpChecker::new(&new_check));
            let sender = Arc::clone(&checker);
            let sender_tx = selector_tx.clone();
            let rcv = thread::spawn(move || {icmp_receiver(&sender, sender_tx)});
            pinger_handles.push(rcv);
            let sender = Arc::clone(&checker);
            let rcv = thread::spawn(move || {icmp_sender(&sender)});
            pinger_handles.push(rcv);
        } else if new_check.check_type == "mtu_icmp" {
            info!("  Starting mtu_icmp for {}", new_check.host);
            let checker = Arc::new(IcmpMtuChecker::new(&new_check));
            let sender = Arc::clone(&checker);
            let sender_tx = selector_tx.clone();
            let rcv = thread::spawn(move || {icmp_mtu_receiver(&sender, sender_tx)});
            pinger_handles.push(rcv);
            let sender = Arc::clone(&checker);
            let rcv = thread::spawn(move || {icmp_mtu_sender(&sender)});
            pinger_handles.push(rcv);
        } else if new_check.check_type == "syn" {
            info!("  Starting syn for {}", new_check.host);
            let checker = Arc::new(SynChecker::new(&new_check));
            let sender = Arc::clone(&checker);
            let sender_tx = selector_tx.clone();
            let rcv = thread::spawn(move || {syn_receiver(&sender, sender_tx)});
            pinger_handles.push(rcv);
            let sender = Arc::clone(&checker);
            let rcv = thread::spawn(move || {syn_sender(&sender)});
            pinger_handles.push(rcv);
        } else if new_check.check_type == "tcp_connect" {
            info!("  Starting tcp_connect for {}", new_check.host);
            let checker = TcpConnectChecker::new(&new_check);
            let sender_tx = selector_tx.clone();
            let rcv = thread::spawn(move || {tcp_connect(checker, sender_tx)});
            pinger_handles.push(rcv);
        } else if new_check.check_type == "udp_server" {
            info!("  Starting udp server for {}", new_check.host);
            let checker = UdpServerChecker::new(&new_check);
            let sender_tx = selector_tx.clone();
            let rcv = thread::spawn(move || {udp_server(checker, sender_tx)});
            pinger_handles.push(rcv);
        } else if new_check.check_type == "udp_client" {
            info!("  Starting udp client for {}", new_check.host);
            let checker = UdpClientChecker::new(&new_check);
            let sender_tx = selector_tx.clone();
            let rcv = thread::spawn(move || {udp_client(checker, sender_tx)});
            pinger_handles.push(rcv);
        } else if new_check.check_type == "remote_listener" {
            info!("  Starting remote_listener");
            let sender_tx = selector_tx.clone();
            let rcv = thread::spawn(move || { run_server(sender_tx) });
            pinger_handles.push(rcv);
        }
        debug!("{:?} {:?} {:?}", &new_check.host, &new_check.check_type, &new_check.interval);
    }

    for handle in pinger_handles {
        handle.join().unwrap();
    };
}
