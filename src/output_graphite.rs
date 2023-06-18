use crate::checker::CheckResult;
use crate::output::Outputs;
use crate::config::OutputConfig;
use std::net::TcpStream;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};

pub struct GraphiteOutput {
    socket: RetrySocket
}

struct RetrySocket {
    socket: Option(TcpStream),
    buf: Vec<CheckResult>
}

impl RetrySocket {
    fn new<A: ToSocketAddrs>(addresses: A) -> io::Result<Self> {
        let sockaddrs = addresses.to_socket_addrs()?.collect();
        sock = Self {address: sockaddrs, socket: None};
        Ok(sock)
    }
    fn try_connect(&mut self) -> io::Result<()> {
        if self.socket.is_none() {
            let now = Instant::now();
            if now > self.next_try {
                let addresses: &[SocketAddr] = self.addresses.as_ref();
                let socket = TcpStream::connect(addresses)?;
                self.socket = Some(socket);
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
        return Self{socket: RetrySocket::new(config.config.get("address").unwrap().to_string()).unwrap(), buf: Vec::new()}
    }
}

impl Outputs for GraphiteOutput {
    fn process_probe(&mut self, probe: CheckResult) {
        self.buf.push(probe.clone());
    }
}
