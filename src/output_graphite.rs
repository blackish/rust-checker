use crate::checker::CheckResult;
use crate::output::Outputs;
use crate::config::OutputConfig;
use std::net::TcpStream;
use std::io;
use std::io::Write;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};
use std::thread::sleep;

const MIN_RECONNECT_DELAY_MS: u64 = 50;
const MAX_RECONNECT_DELAY_MS: u64 = 10_000;

pub struct GraphiteOutput {
    prefix: String,
    names: Vec<String>,
    buffer: Vec<String>,
    socket: RetrySocket
}

struct RetrySocket {
    socket: Option<TcpStream>,
    address: Vec<SocketAddr>,
    next_try: Instant,
    retries: usize
}

impl RetrySocket {
    fn new<A: ToSocketAddrs>(addresses: A) -> io::Result<Self> {
        let sockaddrs = addresses.to_socket_addrs()?.collect();
        let mut sock = Self {
            address: sockaddrs,
            retries: 0,
            next_try: Instant::now(),
            socket: None
        };
        let _ = sock.flush().ok();
        Ok(sock)
    }
    fn try_connect(&mut self) -> io::Result<()> {
        if self.socket.is_none() {
            let now = Instant::now();
            if now < self.next_try {
                sleep(self.next_try - now);
            }
            let addresses: &[SocketAddr] = self.address.as_ref();
            let conn = TcpStream::connect(addresses);
            match conn {
                Ok(socket) => {
                    self.socket = Some(socket);
                    Ok(())
                }
                Err(e) => {
                    self.retries += 1;
                    let exp_delay = MIN_RECONNECT_DELAY_MS << self.retries;
                    let max_delay = MAX_RECONNECT_DELAY_MS.min(exp_delay);
                    self.next_try = now + Duration::from_millis(max_delay);
                    Err(e)
                }
            }
        } else {
            Ok(())
        }
    }

    fn with_socket<F, T>(&mut self, operation: F) -> io::Result<T>
    where
        F: FnOnce(&mut TcpStream) -> io::Result<T>,
    {
        if let Err(_) = self.try_connect() {
            return Err(io::Error::from(io::ErrorKind::NotConnected));
        }

        if let Some(ref mut socket) = self.socket {
            operation(socket)
        } else {
            Err(io::Error::from(io::ErrorKind::NotConnected))
        }
    }
}

impl io::Write for RetrySocket {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.with_socket(|sock: &mut TcpStream| sock.write(buf))
    }

    fn flush(&mut self) -> io::Result<()> {
        self.with_socket(TcpStream::flush)
    }
}


impl GraphiteOutput {
    pub fn new(config: &OutputConfig) -> Self {
        let mut res = Self{
                        socket: RetrySocket::new(config.config.get("address").unwrap().clone().into_string().unwrap()).unwrap(),
                        buffer: Vec::new(),
                        names: Vec::new(),
                        prefix: config.config.get("prefix").unwrap().clone().into_string().unwrap()
                    };
        match config.config.get("names").unwrap() {
            yaml_rust::Yaml::Array(ref s) => {
                for n in s {
                    res.names.push(n.clone().into_string().unwrap());
                }
            },
            _ => {}
        }
        return res;
    }
}

impl Outputs for GraphiteOutput {
    fn process_probe(&mut self, probe: CheckResult) {
        for (key, value) in probe.values {
            let mut received = String::new();
            received += &self.prefix.clone().to_string();
            for l_name in self.names.clone() {
                if let Some(name) = probe.labels.get(&l_name) {
                    received += &(String::from(".") + &name);
                }
            }
            received += &(format!(" {} {} {}\n", key, value, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs().to_string()));
            println!("{:?}", received);
            self.buffer.push(received);
        }
        let to_send: Vec<String> = self.buffer.drain(..).collect();
        for probe_to_send in to_send {
            match self.socket.write_all(probe_to_send.as_bytes()) {
                Ok(()) => {},
                Err(_) => {self.buffer.push(probe_to_send);}
            }
        }
    }
}
