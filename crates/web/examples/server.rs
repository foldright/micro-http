use std::error::Error;
use http::{Method, Request, Response, StatusCode};

use std::sync::Arc;
use async_trait::async_trait;
use matchit::{Params, Router};

use tokio::net::TcpListener;

use micro_http::connection::HttpConnection;

use micro_web::{FnHandler, FnTrait, FromRequest, OptionReqBody, Responder, ResponseBody};
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;
use micro_http::handler::{Handler, make_handler};
use micro_http::protocol::body::ReqBody;
use micro_http::protocol::{ParseError, RequestHeader};

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

    let handler = make_handler(entrance_handler);
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




async fn entrance_handler(mut request: Request<ReqBody>) -> Result<Response<ResponseBody>, Box<dyn Error + Send + Sync>> {

    let mut router = Router::<Box<dyn Handler<ReqBody, Error=Box<dyn Error + Send + Sync>, RespBody=ResponseBody, >>>::new();
    router.insert("/", Box::new(FnHandler::new(simple_handler))).unwrap();
    router.insert("/simple", Box::new(FnHandler::new(simple_handler_2))).unwrap();
    router.insert("/user/:user_id", Box::new(FnHandler::new(get_user))).unwrap();

    let path = request.uri().path();
    let matcher = router.at(path).unwrap();

    let handler = matcher.value;
    let params = matcher.params;

    // request.extensions_mut().insert(params);

    handler.call(request).await

}

async fn simple_handler(method: Method, str: Option<String>, str2: Option<String>) -> String {
    println!("receive body: {}, {}", str.is_some(), str2.is_some());
    format!("1: receive from method: {}\r\n", method)
}

async fn simple_handler_2(method: Method) -> String {
    format!("2: receive from method: {}\r\n", method)
}

async fn get_user(user_id: Param<'_, 0>) -> String {
    format!("received userid: {}", user_id.0)
}


#[async_trait]
impl FromRequest for Param<'_, 0> {

    type Output<'r> = Param<'r, 0>;

    async fn from_request(req: &RequestHeader, _body: OptionReqBody) -> Result<Self::Output<'_>, ParseError> {
        let params = req.extensions().get::<Params>().unwrap();
        let param = params.iter().next().unwrap();
        Ok(Param::<0>(param.1))
    }
}

pub struct Param<'p, const I: usize>(&'p str);