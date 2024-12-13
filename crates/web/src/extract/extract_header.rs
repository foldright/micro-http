//! HTTP header extraction implementations
//! 
//! This module provides extractors for HTTP header-related information from requests,
//! including HTTP methods, header maps and raw request headers. These extractors allow
//! handlers to directly access header information in a type-safe way.
//! 
//! The extractors support both owned and borrowed access to the header data:
//! - Owned extractors like `Method` and `HeaderMap` take ownership of the data
//! - Borrowed extractors like `&Method` and `&HeaderMap` provide reference access
//! 
//! # Examples
//! 
//! ```no_run
//! use http::{HeaderMap, Method};
//! 
//! // Access HTTP method
//! async fn handle_method(method: Method) {
//!     match method {
//!         Method::GET => println!("Handling GET request"),
//!         Method::POST => println!("Handling POST request"),
//!         _ => println!("Handling other request method")
//!     }
//! }
//! 
//! // Access headers
//! async fn handle_headers(headers: &HeaderMap) {
//!     if let Some(content_type) = headers.get("content-type") {
//!         println!("Content-Type: {:?}", content_type);
//!     }
//! }
//! ```

use crate::body::OptionReqBody;
use crate::extract::from_request::FromRequest;
use crate::RequestContext;
use async_trait::async_trait;
use http::{HeaderMap, Method};
use micro_http::protocol::{ParseError, RequestHeader};

/// Extracts the HTTP method by value
/// 
/// This extractor takes ownership of the request method.
#[async_trait]
impl FromRequest for Method {
    type Output<'any> = Method;
    type Error = ParseError;

    async fn from_request(req: &RequestContext, _body: OptionReqBody) -> Result<Self::Output<'static>, Self::Error> {
        Ok(req.method().clone())
    }
}

/// Extracts a reference to the HTTP method
/// 
/// This extractor borrows the request method, avoiding cloning.
#[async_trait]
impl FromRequest for &Method {
    type Output<'r> = &'r Method;
    type Error = ParseError;

    async fn from_request<'r>(req: &'r RequestContext, _body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error> {
        Ok(req.method())
    }
}

/// Extracts a reference to the raw request header
/// 
/// Provides access to the underlying HTTP request header structure.
#[async_trait]
impl FromRequest for &RequestHeader {
    type Output<'r> = &'r RequestHeader;
    type Error = ParseError;

    async fn from_request<'r>(req: &'r RequestContext, _body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error> {
        Ok(req.request_header())
    }
}

/// Extracts a reference to the header map
/// 
/// This extractor provides borrowed access to all HTTP headers.
#[async_trait]
impl FromRequest for &HeaderMap {
    type Output<'r> = &'r HeaderMap;
    type Error = ParseError;

    async fn from_request<'r>(req: &'r RequestContext, _body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error> {
        Ok(req.headers())
    }
}

/// Extracts the header map by value
/// 
/// This extractor clones and takes ownership of all HTTP headers.
#[async_trait]
impl FromRequest for HeaderMap {
    type Output<'any> = HeaderMap;
    type Error = ParseError;

    async fn from_request(req: &RequestContext, _body: OptionReqBody) -> Result<Self::Output<'static>, Self::Error> {
        Ok(req.headers().clone())
    }
}
