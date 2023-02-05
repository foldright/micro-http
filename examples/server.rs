use std::sync::Arc;
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use http::{Request, Response, StatusCode};

use http_body_util::BodyExt;
use tokio::net::TcpListener;
use tiny_http::Result;
use tracing::{error, info, Level, warn};
use tracing_subscriber::FmtSubscriber;
use tiny_http::connection::HttpConnection;
use tiny_http::handler::Handler;
use tiny_http::protocol::body::ReqBody;

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!(port = 8080, "start listening");
    let tcp_listener = match TcpListener::bind("127.0.0.1:8080").await {
        Ok(tcp_listener) => tcp_listener,
        Err(e) => {
            error!(cause = %e, "bind server error");
            return Err(e.into());
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
            let connection = HttpConnection::new(tcp_stream);
            connection.process(handler).await
                .with_context(|| format!("connection process error")).unwrap();
        });
    }
}

struct SimpleHandler;

#[async_trait]
impl Handler for SimpleHandler {
    type RespBody = String;
    type Error = tiny_http::Error;

    async fn handle(&self, request: &mut Request<ReqBody>) -> std::result::Result<Response<Self::RespBody>, Self::Error> {

        let path = request.uri().path().to_string();
        let body = request.body_mut();

        while let Some(Ok(frame)) = body.frame().await {
            let bytes = frame.into_data().map_err(|_e| anyhow!("read request error {}", path)).unwrap();
            info!(body = std::str::from_utf8(&bytes[..]).unwrap(), "receiving request data");
        }

        let body = "Hello World!";

        let response = Response::builder()
            .status(StatusCode::OK)
            .header(http::header::CONTENT_LENGTH, body.len())
            .body(body.to_string())
            .unwrap();

        Ok(response)
    }
}