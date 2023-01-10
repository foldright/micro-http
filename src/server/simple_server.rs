use std::{io, thread};
use std::io::Read;
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use crate::handler::Handler;

pub struct SimpleServer<Addr: ToSocketAddrs> {
    addr: Addr,
}

impl<Addr: ToSocketAddrs> SimpleServer<Addr> {
    pub fn new(addr: Addr) -> Self {
        Self {
            addr,
        }
    }

    pub fn run(&self, handler: Arc<impl Handler>) -> io::Result<()> {
        let listener = TcpListener::bind(&self.addr)?;

        loop {
            let (tcp_stream, _socket_addr) = match listener.accept() {
                Ok((tcp_stream, socket_addr)) => (tcp_stream, socket_addr),
                Err(e) => {
                    eprint!("accept connection error {}", e);
                    continue;
                }
            };

            let handler = Arc::clone(&handler);
            thread::spawn(move || {
                Self::handle(tcp_stream, handler)
            });
        }
    }

    fn handle(mut tcp_stream: TcpStream, handler: Arc<impl Handler>) {

    }
}