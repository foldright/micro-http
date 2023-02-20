use crate::FromRequest;
use futures::future::poll_fn;
use http::request::Request;
use http::Response;
use http_body::Body;
use micro_http::handler::{make_handler, Handler, HandlerFn};
use micro_http::protocol::body::ReqBody;
use std::error::Error;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::FutureExt;
use micro_http::protocol::RequestHeader;

pub trait HttpHandler<Args> {
    type Output;
    type Future: Future<Output = Self::Output>;

    fn call(&self, args: Args) -> Self::Future;
}

#[derive(Debug)]
pub struct HttpHandlerWrapper<F, H> {
    handler: H,
    f: F,
}

pub fn make_htt_handler<F, ReqBody, RespBody, Err, Ret, H, Args>(f: F, h: H) -> HttpHandlerWrapper<F, H>
where
    RespBody: Body,
    Err: Into<Box<dyn Error + Send + Sync>>,
    Ret: Future<Output = Result<Response<RespBody>, Err>>,

    H: HttpHandler<Args>,
    Args: FromRequest,
    H::Output: Into<Response<RespBody>>,

    F: Fn(Request<ReqBody>, H) -> Ret,
{
    HttpHandlerWrapper { handler: h, f }
}

impl<F, RespBody, Err, Ret, H, Args> Handler<ReqBody> for HttpHandlerWrapper<F, H>
where
    RespBody: Body,
    Err: Into<Box<dyn Error + Send + Sync>>,
    Ret: Future<Output = Result<Response<RespBody>, Err>>,

    H: HttpHandler<Args>,
    Args: FromRequest,
    H::Output: Into<Response<RespBody>>,
    F: Fn(Request<ReqBody>, H) -> Ret,
{
    type RespBody = ();
    type Error = ();
    type Future = ();

    fn call(&self, req: Request<ReqBody>) -> Self::Future {
        //(self.f)(req, self.handler)
        todo!()
    }
}

async fn http_handler<H, Args, RespBody>(
    request: Request<ReqBody>,
    http_handler: &H,
) -> Result<Response<RespBody>, Box<dyn Error + Send + Sync>>
where
    H: HttpHandler<Args>,
    Args: FromRequest,
    RespBody: Body,
    H::Output: Into<Response<RespBody>>,
{
    let (parts, mut body) = request.into_parts();
    let request_header = parts.into();

    let args_result: Result<Args, _> = Args::from_request(&request_header, &mut body).await;

    if let Ok(args) = args_result {
        Ok(http_handler.call(args).await.into())
    } else {
        todo!()
    }
}

// adapter fn to HttpHandler
impl<Func, Fut, A, B> HttpHandler<(A, B)> for Func
where
    Func: Fn(A, B) -> Fut,
    Fut: Future,
{
    type Output = Fut::Output;
    type Future = Fut;

    fn call(&self, (a, b): (A, B)) -> Self::Future {
        (self)(a, b)
    }
}

#[cfg(test)]
mod test {
    use crate::HttpHandler;

    fn assert_impl_http_handler<Args, H: HttpHandler<Args>>(_: H) {}

    #[test]
    fn test_basic() {
        async fn two_args(a: (), b: ()) {}

        assert_impl_http_handler(two_args);
    }
}
