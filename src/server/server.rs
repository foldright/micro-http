use tokio::net::{TcpListener, ToSocketAddrs};
use tracing::{error, warn};
use crate::connection::Connection;

pub struct Server<Addr: ToSocketAddrs> {
    addr: Addr,
}

impl<Addr: ToSocketAddrs> Server<Addr> {
    pub fn new(addr: Addr) -> Self {
        Self {
            addr,
        }
    }

    pub async fn run(&self) -> crate::Result<()> {
        let tcp_listener = match TcpListener::bind(&self.addr).await {
            Ok(tcp_listener) => tcp_listener,
            Err(e) => {
                error!(cause = %e, "bind server error");
                return Err(e.into());
            }
        };

        loop {
            let (tcp_stream, _remote_addr) = match tcp_listener.accept().await {
                Ok(stream_and_addr) => stream_and_addr,
                Err(e) => {
                    warn!(cause = %e, "failed to accept");
                    continue;
                }
            };

            let mut connection = Connection::new(tcp_stream);

            let header = connection.read_header::<()>().await?;

            // todo : need continue process the connection
            println!("{header:?}");
        }
    }
}
