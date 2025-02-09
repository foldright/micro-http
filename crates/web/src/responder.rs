//! Response handling module that converts handler results into HTTP responses.
//!
//! This module provides the [`Responder`] trait which defines how different types
//! can be converted into HTTP responses. It includes implementations for common types
//! like Result, Option, String, etc.
//!
//! The [`Responder`] trait is a key part of the response pipeline, allowing handler
//! return values to be automatically converted into proper HTTP responses.

use crate::body::ResponseBody;
use crate::RequestContext;
use http::{Response, StatusCode};
use std::convert::Infallible;

/// A trait for types that can be converted into HTTP responses.
///
/// Types implementing this trait can be returned directly from request handlers
/// and will be automatically converted into HTTP responses.
pub trait Responder {
    fn response_to(self, req: &RequestContext) -> Response<ResponseBody>;
}

/// Implementation for Result allows handlers to return Result types directly.
/// The Ok and Err variants must both implement Responder.
impl<T: Responder, E: Responder> Responder for Result<T, E> {
    fn response_to(self, req: &RequestContext) -> Response<ResponseBody> {
        match self {
            Ok(t) => t.response_to(req),
            Err(e) => e.response_to(req),
        }
    }
}

/// Implementation for Option allows handlers to return Option types.
/// None case returns an empty response.
impl<T: Responder> Responder for Option<T> {
    fn response_to(self, req: &RequestContext) -> Response<ResponseBody> {
        match self {
            Some(t) => t.response_to(req),
            None => Response::new(ResponseBody::empty()),
        }
    }
}

/// Implementation for Response allows passing through pre-built responses.
/// The response body is converted to the internal ResponseBody type.
impl<B> Responder for Response<B>
where
    B: Into<ResponseBody>,
{
    fn response_to(self, _req: &RequestContext) -> Response<ResponseBody> {
        self.map(|b| b.into())
    }
}

/// Implementation for (StatusCode, T) tuple allows setting a status code
/// along with the response content.
impl<T: Responder> Responder for (StatusCode, T) {
    fn response_to(self, req: &RequestContext) -> Response<ResponseBody> {
        let (status, responder) = self;
        let mut response = responder.response_to(req);
        *response.status_mut() = status;
        response
    }
}

/// Implementation for (T, StatusCode) tuple - same as above but with reversed order.
impl<T: Responder> Responder for (T, StatusCode) {
    fn response_to(self, req: &RequestContext) -> Response<ResponseBody> {
        let (responder, status) = self;
        (status, responder).response_to(req)
    }
}

/// Implementation for Box<T> allows boxing responders.
impl<T: Responder> Responder for Box<T> {
    fn response_to(self, req: &RequestContext) -> Response<ResponseBody> {
        (*self).response_to(req)
    }
}

/// Implementation for unit type () returns an empty response.
impl Responder for () {
    fn response_to(self, _req: &RequestContext) -> Response<ResponseBody> {
        Response::new(ResponseBody::empty())
    }
}

/// Implementation for static strings returns them as plain text responses.
impl Responder for &'static str {
    fn response_to(self, _req: &RequestContext) -> Response<ResponseBody> {
        let mut builder = Response::builder();
        let headers = builder.headers_mut().unwrap();
        headers.reserve(8);
        headers.insert(http::header::CONTENT_TYPE, mime::TEXT_PLAIN_UTF_8.as_ref().parse().unwrap());

        builder.status(StatusCode::OK).body(ResponseBody::from(self)).unwrap()
    }
}

/// Implementation for String returns it as a plain text response.
impl Responder for String {
    fn response_to(self, _req: &RequestContext) -> Response<ResponseBody> {
        let mut builder = Response::builder();
        let headers = builder.headers_mut().unwrap();
        headers.reserve(8);
        headers.insert(http::header::CONTENT_TYPE, mime::TEXT_PLAIN_UTF_8.as_ref().parse().unwrap());

        builder.status(StatusCode::OK).body(ResponseBody::from(self)).unwrap()
    }
}

impl Responder for Infallible {
    fn response_to(self, _req: &RequestContext) -> Response<ResponseBody> {
        unreachable!()
    }
}
