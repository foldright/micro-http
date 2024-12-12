//! Request handler types and traits for the web framework.
//! 
//! This module provides the core abstractions for handling HTTP requests:
//! - `RequestHandler` trait for implementing request handlers
//! - `FnHandler` for wrapping async functions as handlers
//! - Helper functions for creating handlers

use crate::body::ResponseBody;
use crate::fn_trait::FnTrait;

use crate::responder::Responder;
use crate::{OptionReqBody, RequestContext};
use async_trait::async_trait;
use http::Response;

use crate::extract::FromRequest;
use std::marker::PhantomData;

/// Trait for types that can handle HTTP requests.
/// 
/// Implementors must provide an `invoke` method that processes the request
/// and returns a response. This is the core trait for request handling.
#[async_trait]
pub trait RequestHandler: Send + Sync {
    async fn invoke<'server, 'req>(
        &self,
        req: &mut RequestContext<'server, 'req>,
        req_body: OptionReqBody,
    ) -> Response<ResponseBody>;
}

#[async_trait]
impl<T> RequestHandler for Box<T>
where
    T: RequestHandler,
{
    async fn invoke<'server, 'req>(
        &self,
        req: &mut RequestContext<'server, 'req>,
        req_body: OptionReqBody,
    ) -> Response<ResponseBody> {
        (**self).invoke(req, req_body).await
    }
}

#[async_trait]
impl RequestHandler for Box<dyn RequestHandler> {
    async fn invoke<'server, 'req>(
        &self,
        req: &mut RequestContext<'server, 'req>,
        req_body: OptionReqBody,
    ) -> Response<ResponseBody> {
        (**self).invoke(req, req_body).await
    }
}

#[async_trait]
impl RequestHandler for &dyn RequestHandler {
    async fn invoke<'server, 'req>(
        &self,
        req: &mut RequestContext<'server, 'req>,
        req_body: OptionReqBody,
    ) -> Response<ResponseBody> {
        (**self).invoke(req, req_body).await
    }
}

/// A wrapper type that converts async functions into request handlers.
/// 
/// This allows regular async functions to be used as request handlers
/// by implementing the `RequestHandler` trait for them.
///
/// Type parameters:
/// - `F`: The async function type
/// - `Args`: The function arguments type
pub struct FnHandler<F, Args> {
    f: F,
    _phantom: PhantomData<fn(Args)>,
}

impl<F, Args> FnHandler<F, Args>
where
    F: FnTrait<Args>,
{
    fn new(f: F) -> Self {
        Self { f, _phantom: PhantomData }
    }
}

/// Creates a new `FnHandler` that wraps the given async function.
///
/// This is the main way to convert an async function into a request handler.
pub fn handler_fn<F, Args>(f: F) -> FnHandler<F, Args>
where
    F: FnTrait<Args>,
{
    FnHandler::new(f)
}

#[async_trait]
impl<F, Args> RequestHandler for FnHandler<F, Args>
where
    // Args must implement [`FromRequest`] trait
    // This allows extracting the function arguments from the HTTP request
    Args: FromRequest,
    // F must be a function [`FnTrait`] that can accept Args::Output<'r> for any lifetime 'r
    // This allows the function to work with arguments that have different lifetimes
    for<'r> F: FnTrait<Args::Output<'r>>,
    // The output of calling F must implement [`Responder`] trait
    // This ensures the function's return value can be converted into an HTTP response
    for<'r> <F as FnTrait<Args::Output<'r>>>::Output: Responder,
{
    async fn invoke<'server, 'req>(
        &self,
        req: &mut RequestContext<'server, 'req>,
        req_body: OptionReqBody,
    ) -> Response<ResponseBody> {
        let args = match Args::from_request(req, req_body.clone()).await {
            Ok(args) => args,
            Err(responder) => return responder.response_to(req),
        };
        let responder = self.f.call(args).await;
        responder.response_to(req)
    }
}

#[cfg(test)]
mod test {
    use crate::fn_trait::FnTrait;
    use crate::handler::{FnHandler, RequestHandler};

    use http::Method;

    fn assert_is_fn_handler<H: FnTrait<Args>, Args>(_handler: &FnHandler<H, Args>) {
        // no op
    }

    fn assert_is_handler<T: RequestHandler>(_handler: &T) {
        // no op
    }

    #[test]
    fn assert_fn_is_http_handler_1() {
        async fn get(_header: Method) {}

        let http_handler = FnHandler::new(get);
        assert_is_fn_handler(&http_handler);
        assert_is_handler(&http_handler);
    }

    #[test]
    fn assert_fn_is_http_handler_2() {
        async fn get(_header: &Method, _str: String) {}

        let http_handler = FnHandler::new(get);
        assert_is_fn_handler(&http_handler);
        assert_is_handler(&http_handler);
    }
}
