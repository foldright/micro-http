use std::error::Error;
use std::future::Future;

use http::{Request, Response};

use http_body::Body;

pub trait Handler<ReqBody> {
    type RespBody: Body;
    type Error: Into<Box<dyn Error + Send + Sync>>;

    async fn call(&self, req: Request<ReqBody>) -> Result<Response<Self::RespBody>, Self::Error>;
}

#[derive(Debug)]
pub struct HandlerFn<F> {
    f: F,
}

impl<ReqBody, RespBody, Err, F, Fut> Handler<ReqBody> for HandlerFn<F>
where
    RespBody: Body,
    F: Fn(Request<ReqBody>) -> Fut,
    Err: Into<Box<dyn Error + Send + Sync>>,
    Fut: Future<Output = Result<Response<RespBody>, Err>>,
{
    type RespBody = RespBody;
    type Error = Err;

    async fn call(&self, req: Request<ReqBody>) -> Result<Response<Self::RespBody>, Self::Error> {
        (self.f)(req).await
    }
}

pub fn make_handler<F, ReqBody, RespBody, Err, Ret>(f: F) -> HandlerFn<F>
where
    RespBody: Body,
    Err: Into<Box<dyn Error + Send + Sync>>,
    Ret: Future<Output = Result<Response<RespBody>, Err>>,
    F: Fn(Request<ReqBody>) -> Ret,
{
    HandlerFn { f }
}
