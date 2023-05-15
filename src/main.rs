extern crate pnet;
pub mod config;
pub mod checker;

use crate::config::{load_config};
use crate::checker::{Checker, icmp_sender, icmp_receiver, CheckResult};

use std::sync::{mpsc, Arc};
use std::thread;

fn main() {
    let cfg = load_config();
    let mut pinger_handles = Vec::<thread::JoinHandle<()>>::new();
    let (mut probe_tx, mut probe_rx) = mpsc::channel();
    for c in cfg.0 {
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

    //let rcv = thread::spawn(move || {
            //let mut iter = ipv4_packet_iter(&mut icmpv4_rx);
            //loop {
                //match iter.next() {
                    //Ok((packet, addr)) => {
                        //let icmp = EchoReplyPacket::new(packet.payload());
                        //println!("{:?} {:?}", addr, icmp);
                    //},
                    //Err(e) => {
                        //println!("An error occurred while reading: {}", e);
                    //}
                //}
            //}
        //});

    //rcv.join();
    for handle in pinger_handles {
        handle.join();
    };
}
