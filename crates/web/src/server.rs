use crate::handler::RequestHandler;

use crate::{OptionReqBody, PathParams, RequestContext, ResponseBody};
use async_trait::async_trait;
use http::{Request, Response};
use matchit::Router;
use micro_http::connection::HttpConnection;
use micro_http::handler::Handler;
use micro_http::protocol::body::ReqBody;
use micro_http::protocol::RequestHeader;
use std::error::Error;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

pub struct Server {
    router: Router<Box<dyn RequestHandler>>,
    default_handler: Option<Box<dyn RequestHandler>>,
}

pub struct ServerBuilder {
    router: Router<Box<dyn RequestHandler>>,
    default_handler: Option<Box<dyn RequestHandler>>,
}

impl Server {
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    pub async fn start(self) {
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

        let handler = Arc::new(self);
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
}

impl ServerBuilder {
    fn new() -> Self {
        Self { router: Router::new(), default_handler: None }
    }

    pub fn route(mut self, path: impl Into<String>, request_handler: impl RequestHandler + 'static) -> Self {
        self.router.insert(path, Box::new(request_handler)).unwrap();
        self
    }

    pub fn default_handler(mut self, request_handler: impl RequestHandler + 'static) -> Self {
        self.default_handler = Some(Box::new(request_handler));
        self
    }

    pub fn build(self) -> Server {
        Server { router: self.router, default_handler: self.default_handler }
    }
}

#[async_trait]
impl Handler for Server {
    type RespBody = ResponseBody;
    type Error = Box<dyn Error + Send + Sync>;

    async fn call(&self, req: Request<ReqBody>) -> Result<Response<Self::RespBody>, Self::Error> {
        let (parts, body) = req.into_parts();
        let header = RequestHeader::from(parts);
        let req_body = OptionReqBody::from(body);

        let path = header.uri().path();
        let matcher = match self.router.at(path) {
            Ok(matcher) => matcher,
            Err(_e) => {
                let request_context = RequestContext::new(&header, PathParams::empty());
                // todo: do not using unwrap
                let default_handler = self.default_handler.as_ref().unwrap();
                return default_handler.invoke(request_context, req_body).await;
            }
        };

        let params = matcher.params;
        let handler = matcher.value;
        let request_context = RequestContext::new(&header, params.into());

        handler.invoke(request_context, req_body).await
    }
}
