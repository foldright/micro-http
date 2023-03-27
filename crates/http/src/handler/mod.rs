use crate::protocol::body::ReqBody;
use http::{Request, Response};
use http_body::Body;
use std::error::Error;
use std::future::Future;

pub trait Handler: Send + Sync {
    type RespBody: Body;
    type Error: Into<Box<dyn Error + Send + Sync>>;
    type Fut<'fut>: Future<Output = Result<Response<Self::RespBody>, Self::Error>>
    where
        Self: 'fut;

    fn call(&self, req: Request<ReqBody>) -> Self::Fut<'_>;
}

#[derive(Debug)]
pub struct HandlerFn<F> {
    f: F,
}

impl<RespBody, Err, F, Fut> Handler for HandlerFn<F>
where
    RespBody: Body,
    F: Fn(Request<ReqBody>) -> Fut + Send + Sync,
    Err: Into<Box<dyn Error + Send + Sync>>,
    Fut: Future<Output = Result<Response<RespBody>, Err>> + Send,
{
    type RespBody = RespBody;
    type Error = Err;
    type Fut<'fut> = Fut where Self: 'fut;

    fn call(&self, req: Request<ReqBody>) -> Self::Fut<'_> {
        (self.f)(req)
    }
}

pub fn make_handler<F, RespBody, Err, Ret>(f: F) -> HandlerFn<F>
where
    RespBody: Body,
    Err: Into<Box<dyn Error + Send + Sync>>,
    Ret: Future<Output = Result<Response<RespBody>, Err>>,
    F: Fn(Request<ReqBody>) -> Ret,
{
    HandlerFn { f }
}
