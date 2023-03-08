use http::{header, HeaderMap, HeaderName, Method, Request, Response, StatusCode};
use http_body_util::BodyExt;
use std::error::Error;
use std::sync::Arc;
use http_body_util::combinators::BoxBody;

use tokio::net::TcpListener;

use micro_http::connection::HttpConnection;
use micro_http::handler::{Handler, make_handler};
use micro_http::protocol::body::ReqBody;
use micro_http::protocol::RequestHeader;
use micro_web::FnHandler;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder().with_max_level(Level::INFO).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!(port = 8080, "start listening");
    let tcp_listener = match TcpListener::bind("127.0.0.1:8080").await {
        Ok(tcp_listener) => tcp_listener,
        Err(e) => {
            error!(cause = %e, "bind server error");
            return;
        }
    };

    let handler = FnHandler::new(simple_handler);
    let handler = Arc::new(handler);
    loop {
        let (tcp_stream, _remote_addr) = match tcp_listener.accept().await {
            Ok(stream_and_addr) => stream_and_addr,
            Err(e) => {
                warn!(cause = %e, "failed to accept");
                continue;
            }
        };

        let handler = handler.clone();

        tokio::spawn(async move {
            let (reader, writer) = tcp_stream.into_split();
            let connection = HttpConnection::new(reader, writer);
            match connection.process(handler).await {
                Ok(_) => {
                    info!("finished process, connection shutdown");
                }
                Err(e) => {
                    error!("service has error, cause {}, connection shutdown", e);
                }
            }
        });
    }
}

async fn simple_handler(method: Method) -> String {
    format!("receive from method: {}\r\n", method)
}