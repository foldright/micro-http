//! Server module for handling HTTP requests and managing web server lifecycle.
//!
//! This module provides the core server functionality including:
//! - Server builder pattern for configuration
//! - HTTP request routing and handling
//! - Connection management and error handling
//! - Default request handling
//!
//! # Examples
//!
//! ```no_run
//! use micro_web::{Server, router::{Router, get}};
//!
//! async fn hello_world() -> &'static str {
//!     "Hello, World!"
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let router = Router::builder()
//!         .route("/", get(hello_world))
//!         .build();
//!         
//!     Server::builder()
//!         .router(router)
//!         .bind("127.0.0.1:3000")
//!         .build()
//!         .unwrap()
//!         .start()
//!         .await;
//! }
//! ```

use crate::handler::RequestHandler;
use crate::router::Router;
use crate::{OptionReqBody, RequestContext, ResponseBody, handler_fn, FnTrait};
use http::{Request, Response, StatusCode};
use micro_http::connection::HttpConnection;
use micro_http::handler::Handler;
use micro_http::protocol::RequestHeader;
use micro_http::protocol::body::ReqBody;
use std::error::Error;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use thiserror::Error;
use tokio::net::TcpListener;
use tracing::{Level, error, info, warn};
use tracing_subscriber::FmtSubscriber;
use crate::extract::FromRequest;
use crate::responder::Responder;

/// Builder for configuring and constructing a [`Server`] instance.
///
/// The builder provides a fluent API for setting server options including:
/// - Binding address
/// - Request router
/// - Default request handler
#[derive(Debug)]
pub struct ServerBuilder {
    router: Option<Router>,
    default_handler: Option<Box<dyn RequestHandler>>,
    address: Option<Vec<SocketAddr>>,
}

impl ServerBuilder {
    fn new() -> Self {
        Self { router: None, default_handler: None, address: None }
    }

    pub fn bind<A: ToSocketAddrs>(mut self, address: A) -> Self {
        self.address = Some(address.to_socket_addrs().unwrap().collect::<Vec<_>>());
        self
    }

    pub fn router(mut self, router: Router) -> Self {
        self.router = Some(router);
        self
    }

    pub fn default_handler<F, Args>(mut self, f: F) -> Self
    where
    for<'r> F: FnTrait<Args> + 'r,
    for<'r> Args: FromRequest + 'r,
    for<'r> F: FnTrait<Args::Output<'r>>,
    for<'r> <F as FnTrait<Args::Output<'r>>>::Output: Responder,
    {
        let handler = handler_fn(f);
        self.default_handler = Some(Box::new(handler));
        self
    }

    pub fn build(self) -> Result<Server, ServerBuildError> {
        let new_builder = if self.default_handler.is_none() { self.default_handler(default_handler) } else { self };
        let router = new_builder.router.ok_or(ServerBuildError::MissingRouter)?;
        let address = new_builder.address.ok_or(ServerBuildError::MissingAddress)?;

        // unwrap is safe here because we set it in the new_builder
        Ok(Server { router, default_handler: new_builder.default_handler.unwrap(), address })
    }
}

async fn default_handler() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "404 Not Found")
}

/// Core server implementation that processes HTTP requests.
///
/// The server is responsible for:
/// - Listening for incoming connections
/// - Routing requests to appropriate handlers
/// - Managing connection lifecycle
/// - Error handling and logging
///
#[derive(Debug)]
pub struct Server {
    router: Router,
    default_handler: Box<dyn RequestHandler>,
    address: Vec<SocketAddr>,
}

/// Errors that can occur during server construction.
#[derive(Error, Debug)]
pub enum ServerBuildError {
    /// Router was not configured
    #[error("router must be set")]
    MissingRouter,

    /// Bind address was not configured
    #[error("address must be set")]
    MissingAddress,
}

impl Server {
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    pub async fn start(self) {
        let subscriber = FmtSubscriber::builder().with_max_level(Level::WARN).finish();
        tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

        info!("start listening at {:?}", self.address);
        let tcp_listener = match TcpListener::bind(self.address.as_slice()).await {
            Ok(tcp_listener) => tcp_listener,
            Err(e) => {
                error!(cause = %e, "bind server error");
                return;
            }
        };

        let handler = Arc::new(self);
        loop {
            let (tcp_stream, _remote_addr) = tokio::select! {
                _ = tokio::signal::ctrl_c() => { break; },
                result = tcp_listener.accept() => {
                    match result {
                        Ok(stream_and_addr) => stream_and_addr,
                        Err(e) => {
                            warn!(cause = %e, "failed to accept");
                            continue;
                        }
                    }
                }
            };

            let handler = handler.clone();

            tokio::spawn(async move {
                tcp_stream.set_nodelay(true).unwrap();
                let (reader, writer) = tcp_stream.into_split();
                let connection = HttpConnection::new(reader, writer);
                match connection.process(handler.as_ref()).await {
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

impl Handler for Server {
    type RespBody = ResponseBody;
    type Error = Box<dyn Error + Send + Sync>;

    async fn call(&self, req: Request<ReqBody>) -> Result<Response<Self::RespBody>, Self::Error> {
        let (parts, body) = req.into_parts();
        let header = RequestHeader::from(parts);
        // TODO: insignificant memory allocate
        let req_body = OptionReqBody::from(body);

        let path = header.uri().path();
        let route_result = self.router.at(path);

        let mut request_context = RequestContext::new(&header, route_result.params());

        let handler = route_result
            .router_items()
            .iter()
            .filter(|item| item.filter().matches(&request_context))
            .map(|item| item.handler())
            .take(1)
            .next()
            .unwrap_or(self.default_handler.as_ref());

        let response = handler.invoke(&mut request_context, req_body).await;
        Ok(response)
    }
}
