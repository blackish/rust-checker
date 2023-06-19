use crate::checker::CheckResult;
use crate::output::Outputs;
use crate::config::OutputConfig;
use std::net::TcpStream;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};
use std::thread::sleep;

const MIN_RECONNECT_DELAY_MS: u64 = 50;
const MAX_RECONNECT_DELAY_MS: u64 = 10_000;

pub struct GraphiteOutput {
    prefix: String,
    name: String,
    buffer: Vec<String>,
    socket: RetrySocket
}

struct RetrySocket {
    socket: Option(TcpStream),
    address: Vec<SocketAddr>,
    next_try: Instant,
    retries: usize
}

impl RetrySocket {
    fn new<A: ToSocketAddrs>(addresses: A) -> io::Result<Self> {
        let sockaddrs = addresses.to_socket_addrs()?.collect();
        sock = Self {
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
            let addresses: &[SocketAddr] = self.addresses.as_ref();
            let conn = TcpStream::connect(addresses);
            match conn {
                Ok(socket) => {
                    self.socket = socket;
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
        }
        Ok(())
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
        };
    }
    fn flush(&mut self) -> Result<> {
        self.with_socket(TcpStream::flush)
    }
    fn write(&mut self, &mut buf [u8]) -> Result<usize> {
        self.with_socket(|sock| sock.write(buf))
    }
}

impl Write for RetrySocket {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.with_socket(|sock| sock.write(buf))
    }

    fn flush(&mut self) -> io::Result<()> {
        self.with_socket(TcpStream::flush)
    }
}


impl GraphiteOutput {
    pub fn new(config: &OutputConfig) -> Self {
        Self{
            socket: RetrySocket::new(config.config.get("address").unwrap().to_string()).unwrap(),
            buffer: Vec::new(),
            prefix: config.config.get("prefix").unwrap().to_string()
        }
    }
}

impl Outputs for GraphiteOutput {
    fn process_probe(&mut self, probe: CheckResult) {
        for (key, value) in probe.values {
            let mut received = self.prefix.clone();
            if self.name != "" && let Some(name) == probe.labels.get(self.name) {
                received += name + "_"
            }
            received += format!("{} {} {}\n", key, value, SystemTime.now().duration_since(UNIX_EPOCH).unwrap().as_secs().to_string());
            self.buf.push(received);
        }
        let to_send = self.buf.drain().collect();
        for probe in to_send {
            match self.sock.write_all(probe.as_byte()) {
                Ok(()) => {},
                Err(_) => {self.buf.push(probe);}
            }
        }
    }
}
