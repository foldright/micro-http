use std::{io, thread};
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use crate::handler::Handler;
use crate::protocol::Request;
use crate::server::ServerError;
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
            let (tcp_stream, socket_addr) = match listener.accept() {
                Ok((tcp_stream, socket_addr)) => (tcp_stream, socket_addr),
                Err(e) => {
                    eprint!("accept connection error {}", e);
                    continue;
                }
            };

            let handler = Arc::clone(&handler);
            thread::spawn(move || {
                Self::handle(tcp_stream, socket_addr, handler)
            });
        }
    }

    fn handle(mut tcp_stream: TcpStream, socket_addr: SocketAddr, handler: Arc<impl Handler>) {
        const BUF_SIZE: usize = 1024;
        let mut buf = [0u8; BUF_SIZE];
        let read_size = match read(&mut tcp_stream, &mut buf) {
            Ok(len) => len,
            Err(e) => {
                eprintln!("read socket [{}] error [{}]", socket_addr, e);
                tcp_stream.shutdown(Shutdown::Both).unwrap();
                return;
            }
        };

        let buf = &buf[..read_size];

        let request = Request::try_from(buf).unwrap();

        let response = handler.handle(&request);

        response.write(&mut tcp_stream);
    }
}

fn read(reader: &mut impl Read, buf: &mut [u8]) -> Result<usize, ServerError> {
    match reader.read(buf) {
        Ok(len) if len == 0 => Err(ServerError::BrokenConnection),
        Ok(len) if len == buf.len() => Err(ServerError::BufSizeFull),
        Ok(len) => Ok(len),
        Err(e) => Err(ServerError::IoError(e.kind())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_new() {
        let simple_server = SimpleServer::new("127.0.0.1:8080");
        assert_eq!(simple_server.addr, "127.0.0.1:8080");
    }

    #[test]
    fn test_read_zero_bytes() {
        let mut reader = io::empty();
        let mut buf = [0u8; 16];

        let result = read(&mut reader, &mut buf);
        assert_eq!(result, Err(ServerError::BrokenConnection));
    }

    #[test]
    fn test_read_normal_bytes() {
        let mut reader = io::repeat(0u8).take(15);
        let mut buf = [0u8; 16];

        let result = read(&mut reader, &mut buf);
        assert_eq!(result, Ok(15));
    }

    #[test]
    fn test_read_full_bytes_1() {
        let mut reader = io::repeat(0u8).take(16);
        let mut buf = [0u8; 16];

        let result = read(&mut reader, &mut buf);
        assert_eq!(result, Err(ServerError::BufSizeFull));
    }

    #[test]
    fn test_read_full_bytes_2() {
        let mut reader = io::repeat(0u8).take(17);
        let mut buf = [0u8; 16];

        let result = read(&mut reader, &mut buf);
        assert_eq!(result, Err(ServerError::BufSizeFull));
    }
}