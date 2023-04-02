use crate::handler::RequestHandler;
use std::error::Error;
use std::future::Future;

use crate::interceptor::{Interceptor, Interceptors};
use crate::router::Router;
use crate::{OptionReqBody, RequestContext, ResponseBody};
use http::{Request, Response};
use micro_http::connection::HttpConnection;
use micro_http::handler::Handler;
use micro_http::protocol::body::ReqBody;
use micro_http::protocol::RequestHeader;
use std::net::{SocketAddr, ToSocketAddrs};
use std::pin::Pin;
use std::sync::Arc;
use thiserror::Error;
use tokio::net::TcpListener;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

pub struct ServerBuilder {
    router: Option<Router>,
    default_handler: Option<Box<dyn RequestHandler>>,
    address: Option<Vec<SocketAddr>>,
    interceptors: Interceptors,
}

impl ServerBuilder {
    fn new() -> Self {
        Self { router: None, default_handler: None, address: None, interceptors: Interceptors::builder().build() }
    }

    pub fn address<A: ToSocketAddrs>(mut self, address: A) -> Self {
        self.address = Some(address.to_socket_addrs().unwrap().collect::<Vec<_>>());
        self
    }

    pub fn router(mut self, router: Router) -> Self {
        self.router = Some(router);
        self
    }

    pub fn default_handler(mut self, request_handler: impl RequestHandler + 'static) -> Self {
        self.default_handler = Some(Box::new(request_handler));
        self
    }

    pub fn interceptors(mut self, interceptors: Interceptors) -> Self {
        self.interceptors = interceptors;
        self
    }

    pub fn build(self) -> Result<Server, ServerBuildError> {
        let router = self.router.ok_or(ServerBuildError::MissingRouter).unwrap();
        let address = self.address.ok_or(ServerBuildError::MissingAddress).unwrap();
        Ok(Server { router, default_handler: self.default_handler, address, interceptors: self.interceptors })
    }
}

pub struct Server {
    router: Router,
    default_handler: Option<Box<dyn RequestHandler>>,
    address: Vec<SocketAddr>,
    interceptors: Interceptors,
}

#[derive(Error, Debug)]
pub enum ServerBuildError {
    #[error("router must be set")]
    MissingRouter,
    #[error("address must be set")]
    MissingAddress,
}

impl Server {
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    pub async fn start(self) {
        let subscriber = FmtSubscriber::builder().with_max_level(Level::INFO).finish();
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

impl Handler for Server {
    type RespBody = ResponseBody;
    type Error = Box<dyn Error + Send + Sync>;
    type Fut<'fut> = Pin<Box<dyn Future<Output = Result<Response<Self::RespBody>, Self::Error>> + Send + 'fut>>;

    fn call(&self, req: Request<ReqBody>) -> Self::Fut<'_> {
        Box::pin(async {
            let (parts, body) = req.into_parts();
            let header = RequestHeader::from(parts);
            let mut req_body = OptionReqBody::from(body);

            let path = header.uri().path();
            let route_result = self.router.at(path);

            let mut request_context = RequestContext::new(&header, route_result.params());

            let handler_option = route_result
                .router_items()
                .into_iter()
                .filter(|item| item.filter().check(&request_context))
                .map(|item| item.handler())
                .take(1)
                .next();

            match handler_option {
                Some(handler) => {
                    self.interceptors.on_request(&mut request_context, &mut req_body).await;
                    let mut response = handler.invoke(&mut request_context, req_body).await;
                    self.interceptors.on_response(&request_context, &mut response).await;
                    Ok(response)
                }
                None => {
                    // todo: do not using unwrap
                    let default_handler = self.default_handler.as_ref().unwrap();

                    self.interceptors.on_request(&mut request_context, &mut req_body).await;
                    let mut response = default_handler.invoke(&mut request_context, req_body).await;
                    self.interceptors.on_response(&request_context, &mut response).await;
                    Ok(response)
                }
            }
        })
    }
}
