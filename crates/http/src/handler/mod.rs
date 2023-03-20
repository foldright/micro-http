use std::error::Error;
use std::future::Future;
use async_trait::async_trait;

use http::{Request, Response};

use http_body::Body;

#[async_trait]
pub trait Handler<ReqBody> {
    type RespBody: Body;
    type Error: Into<Box<dyn Error + Send + Sync>>;

    async fn call(&self, req: Request<ReqBody>) -> Result<Response<Self::RespBody>, Self::Error>;
}

#[derive(Debug)]
pub struct HandlerFn<F> {
    f: F,
}

#[async_trait]
impl<ReqBody, RespBody, Err, F, Fut> Handler<ReqBody> for HandlerFn<F>
where
    RespBody: Body,
    ReqBody: Send + 'static,
    F: Fn(Request<ReqBody>) -> Fut + Send + Sync,
    Err: Into<Box<dyn Error + Send + Sync>>,
    Fut: Future<Output = Result<Response<RespBody>, Err>> + Send,
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
