//! HTTP request handler module
//!
//! This module provides traits and utilities for handling HTTP requests. It defines
//! the core abstraction for request processing through the [`Handler`] trait and
//! provides utilities for creating handlers from async functions.
//!
//! # Examples
//!
//! ```no_run
//! use micro_http::handler::{Handler, make_handler};
//! use micro_http::protocol::body::ReqBody;
//! use http::{Request, Response};
//! use std::error::Error;
//!
//! // Define an async handler function
//! async fn hello_handler(req: Request<ReqBody>) -> Result<Response<String>, Box<dyn Error + Send + Sync>> {
//!     Ok(Response::new("Hello World!".to_string()))
//! }
//!
//! // Create a handler from the function
//! let handler = make_handler(hello_handler);
//! ```


use crate::protocol::body::ReqBody;
use http::{Request, Response};
use http_body::Body;
use std::error::Error;
use std::future::Future;

/// A trait for handling HTTP requests
///
/// This trait defines the core interface for processing HTTP requests and generating responses.
/// Implementors of this trait can be used to handle requests in the HTTP server.
///
/// # Type Parameters
///
/// * `RespBody`: The response body type that implements [`Body`]
/// * `Error`: The error type that can be converted into a boxed error
/// * `Fut`: The future type returned by the handler
#[trait_variant::make(Handler: Send)]
pub trait LocalHandler: Sync {
    /// The type of the response body
    type RespBody: Body;

    /// The error type returned by the handler
    type Error: Into<Box<dyn Error + Send + Sync>>;

    async fn call(&self, req: Request<ReqBody>) -> Result<Response<Self::RespBody>, Self::Error>;
}

/// A wrapper type for function-based handlers
///
/// This type implements [`Handler`] for functions that take a [`Request`] and
/// return a [`Future`] resolving to a [`Response`].
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

    async fn call(&self, req: Request<ReqBody>) -> Result<Response<Self::RespBody>, Self::Error> {
        (self.f)(req).await
    }
}

/// Creates a new handler from an async function
///
/// This function wraps an async function in a [`HandlerFn`] type that implements
/// the [`Handler`] trait.
///
/// # Arguments
///
/// * `f` - An async function that takes a [`Request`] and returns a [`Future`]
///         resolving to a [`Response`]
///
/// # Examples
///
/// ```no_run
/// use micro_http::handler::make_handler;
/// use http::{Request, Response};
/// use micro_http::protocol::body::ReqBody;
/// use std::error::Error;
///
/// async fn my_handler(req: Request<ReqBody>) -> Result<Response<String>, Box<dyn Error + Send + Sync>> {
///     Ok(Response::new("Hello".to_string()))
/// }
///
/// let handler = make_handler(my_handler);
/// ```
pub fn make_handler<F, RespBody, Err, Ret>(f: F) -> HandlerFn<F>
where
    RespBody: Body,
    Err: Into<Box<dyn Error + Send + Sync>>,
    Ret: Future<Output = Result<Response<RespBody>, Err>>,
    F: Fn(Request<ReqBody>) -> Ret,
{
    HandlerFn { f }
}
