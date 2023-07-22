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

use log::info;
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
    for mut p in cfg.1 {
        if p.process_name == "stats_count" {
            info!("Starting stats_count");
            let (process_tx, process_rx) = mpsc::channel();
            p.sender = Some(process_tx);
            let to_selector = selector_tx.clone();
            let mut processor = StatsCount::new(&p, to_selector, process_rx);
            let rcv = thread::spawn(move || { processor.process_probe() });
            pinger_handles.push(rcv);
            processes.push(p);
        } else if p.process_name == "stats_time" {
            info!("  Starting stats_time");
            let (process_tx, process_rx) = mpsc::channel();
            p.sender = Some(process_tx);
            let to_selector = selector_tx.clone();
            let mut processor = StatsTime::new(&p, to_selector, process_rx);
            let rcv = thread::spawn(move || { processor.process_probe() });
            pinger_handles.push(rcv);
            processes.push(p);
        } else if p.process_name == "histogram" {
            info!("  Starting histogram");
            let (process_tx, process_rx) = mpsc::channel();
            p.sender = Some(process_tx);
            let to_selector = selector_tx.clone();
            let mut processor = Histogram::new(&p, to_selector, process_rx);
            let rcv = thread::spawn(move || { processor.process_probe() });
            pinger_handles.push(rcv);
            processes.push(p);
        }
    }
    let mut outputs = Vec::new();
    info!("Starting outputs");
    for mut o in cfg.2 {
        if o.output_name == "print" {
            info!("  Starting print");
            let output = PrintOutput::new(o.id.clone());
            let (output_tx, output_rx) = mpsc::channel();
            let rcv = thread::spawn(move || { output_worker(output, output_rx) });
            pinger_handles.push(rcv);
            o.sender = Some(output_tx);
            outputs.push(o);
        } else if o.output_name == "remote_sender" {
            info!("  Starting remote_sender");
            let output = RemoteOutput::new(&o);
            let (output_tx, output_rx) = mpsc::channel();
            let rcv = thread::spawn(move || { output_worker(output, output_rx) });
            pinger_handles.push(rcv);
            o.sender = Some(output_tx);
            outputs.push(o);
        } else if o.output_name == "graphite" {
            info!("  Starting graphite");
            let output = GraphiteOutput::new(&o);
            let (output_tx, output_rx) = mpsc::channel();
            let rcv = thread::spawn(move || { output_worker(output, output_rx) });
            pinger_handles.push(rcv);
            o.sender = Some(output_tx);
            outputs.push(o);
        }
    }
    let rcv = thread::spawn(move || { selector_worker(selector_rx, processes, outputs) });
    pinger_handles.push(rcv);
    let hosts = cfg.0;
    info!("Starting probes");
    for c in hosts {
        if c.check_type == "icmp" {
            info!("  Starting icmp for {}", c.host);
            let checker = Arc::new(IcmpChecker::new(&c));
            let sender = Arc::clone(&checker);
            let sender_tx = selector_tx.clone();
            let rcv = thread::spawn(move || {icmp_receiver(&sender, sender_tx)});
            pinger_handles.push(rcv);
            let sender = Arc::clone(&checker);
            let rcv = thread::spawn(move || {icmp_sender(&sender)});
            pinger_handles.push(rcv);
        } else if c.check_type == "mtu_icmp" {
            info!("  Starting mtu_icmp for {}", c.host);
            let checker = Arc::new(IcmpMtuChecker::new(&c));
            let sender = Arc::clone(&checker);
            let sender_tx = selector_tx.clone();
            let rcv = thread::spawn(move || {icmp_mtu_receiver(&sender, sender_tx)});
            pinger_handles.push(rcv);
            let sender = Arc::clone(&checker);
            let rcv = thread::spawn(move || {icmp_mtu_sender(&sender)});
            pinger_handles.push(rcv);
        } else if c.check_type == "syn" {
            info!("  Starting syn for {}", c.host);
            let checker = Arc::new(SynChecker::new(&c));
            let sender = Arc::clone(&checker);
            let sender_tx = selector_tx.clone();
            let rcv = thread::spawn(move || {syn_receiver(&sender, sender_tx)});
            pinger_handles.push(rcv);
            let sender = Arc::clone(&checker);
            let rcv = thread::spawn(move || {syn_sender(&sender)});
            pinger_handles.push(rcv);
        } else if c.check_type == "tcp_connect" {
            info!("  Starting tcp_connect for {}", c.host);
            let checker = TcpConnectChecker::new(&c);
            let sender_tx = selector_tx.clone();
            let rcv = thread::spawn(move || {tcp_connect(checker, sender_tx)});
            pinger_handles.push(rcv);
        } else if c.check_type == "udp_server" {
            info!("  Starting udp server for {}", c.host);
            let checker = UdpServerChecker::new(&c);
            let sender_tx = selector_tx.clone();
            let rcv = thread::spawn(move || {udp_server(checker, sender_tx)});
            pinger_handles.push(rcv);
        } else if c.check_type == "udp_client" {
            info!("  Starting udp client for {}", c.host);
            let checker = UdpClientChecker::new(&c);
            let sender_tx = selector_tx.clone();
            let rcv = thread::spawn(move || {udp_client(checker, sender_tx)});
            pinger_handles.push(rcv);
        } else if c.check_type == "remote_listener" {
            info!("  Starting remote_listener");
            let sender_tx = selector_tx.clone();
            let rcv = thread::spawn(move || { run_server(sender_tx) });
            pinger_handles.push(rcv);
        }
        println!("{:?} {:?} {:?}", &c.host, &c.check_type, &c.interval);
    }

    for handle in pinger_handles {
        handle.join().unwrap();
    };
}
