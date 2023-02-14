use async_trait::async_trait;
use http::{Request, Response, StatusCode};
use std::error::Error;
use std::sync::Arc;
use http_body_util::BodyExt;

use tiny_http::connection::HttpConnection;
use tiny_http::handler::Handler;
use tiny_http::protocol::body::ReqBody;
use tokio::net::TcpListener;

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

    let handler = SimpleHandler;
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

struct SimpleHandler;

#[async_trait]
impl Handler for SimpleHandler {
    type RespBody = String;
    type Error = Box<dyn Error + Send>;

    async fn handle(&self, request: Request<ReqBody>) -> Result<Response<Self::RespBody>, Self::Error> {
        let _path = request.uri().path().to_string();
        let (_header, body) = request.into_parts();

        let body_bytes = match body.collect().await {
            Ok(b) => b.to_bytes(),
            Err(err) => {
                return Err(Box::new(err));
            }
        };
        info!(body = std::str::from_utf8(&body_bytes[..]).unwrap(), "receiving request body");

        let response_body = "Hello World!\r\n";

        let response = Response::builder()
            .status(StatusCode::OK)
            .header(http::header::CONTENT_LENGTH, response_body.len())
            .body(response_body.to_string())
            .unwrap();

        Ok(response)
    }
}
