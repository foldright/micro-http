use anyhow::Context;
use async_trait::async_trait;
use http::{Request, Response, StatusCode};
use std::sync::Arc;

use tiny_http::connection::HttpConnection;
use tiny_http::handler::Handler;
use tiny_http::protocol::body::ReqBody;
use tiny_http::Result;
use tokio::net::TcpListener;

use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder().with_max_level(Level::INFO).finish();

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
            let (reader, writer) = tcp_stream.into_split();
            let connection = HttpConnection::new(reader, writer);
            connection.process(handler).await.with_context(|| format!("connection process error")).unwrap();
        });
    }
}

struct SimpleHandler;

#[async_trait]
impl Handler for SimpleHandler {
    type RespBody = String;
    type Error = tiny_http::Error;

    async fn handle(&self, request: Request<ReqBody>) -> std::result::Result<Response<Self::RespBody>, Self::Error> {
        let _path = request.uri().path().to_string();
        let (_header, _body) = request.into_parts();

        // let body = body.collect().await?;
        // info!(body = std::str::from_utf8(&body.to_bytes()[..]).unwrap(), "receiving request body");

        let body = "Hello World!";

        //sleep(Duration::from_secs(10)).await;

        let response = Response::builder()
            .status(StatusCode::OK)
            .header(http::header::CONTENT_LENGTH, body.len())
            .body(body.to_string())
            .unwrap();

        Ok(response)
    }
}
