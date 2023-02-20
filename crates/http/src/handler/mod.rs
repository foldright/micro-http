use std::error::Error;
use std::future::Future;

use http::{Request, Response};

use http_body::Body;

pub trait Handler<ReqBody> {
    type RespBody: Body;

    type Error: Into<Box<dyn Error + Send + Sync>>;

    type Future: Future<Output = Result<Response<Self::RespBody>, Self::Error>>;

    fn call(&self, req: Request<ReqBody>) -> Self::Future;
}

#[derive(Debug)]
pub struct HandlerFn<F> {
    f: F,
}

impl<F, ReqBody, RespBody, Err, Ret> Handler<ReqBody> for HandlerFn<F>
where
    RespBody: Body,
    Err: Into<Box<dyn Error + Send + Sync>>,
    Ret: Future<Output = Result<Response<RespBody>, Err>>,
    F: Fn(Request<ReqBody>) -> Ret,
{
    type RespBody = RespBody;
    type Error = Err;
    type Future = Ret;

    fn call(&self, req: Request<ReqBody>) -> Self::Future {
        (self.f)(req)
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
