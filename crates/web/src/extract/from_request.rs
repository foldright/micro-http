//! Typed data extraction from HTTP requests
//!
//! This module provides the [`FromRequest`] trait which enables extracting typed data
//! from HTTP requests. It serves as the foundation for the parameter extraction system,
//! allowing handlers to receive strongly-typed parameters.
//!
//! # Examples
//!
//! ```ignore
//! use micro_web::extract::FromRequest;
//! use micro_web::RequestContext;
//! use async_trait::async_trait;
//! use micro_http::protocol::ParseError;
//! use crate::body::OptionReqBody;
//!
//! struct CustomType(String);
//!
//! #[async_trait]
//! impl FromRequest for CustomType {
//!     type Output<'r> = CustomType;
//!     type Error = ParseError;
//!
//!     async fn from_request<'r>(
//!         req: &'r RequestContext,
//!         body: OptionReqBody
//!     ) -> Result<Self::Output<'r>, Self::Error> {
//!         // Extract and validate data from request
//!         let data = String::from_request(req, body).await?;
//!         Ok(CustomType(data))
//!     }
//! }
//! ```

use crate::body::OptionReqBody;
use crate::responder::Responder;
use crate::{RequestContext, ResponseBody};
use async_trait::async_trait;
use http::{Response, StatusCode};
use micro_http::protocol::ParseError;

/// Trait for extracting typed data from an HTTP request
///
/// This trait enables request handlers to receive typed parameters extracted from
/// the request. It supports extracting data from:
/// - Request headers
/// - URL parameters
/// - Query strings
/// - Request body (as JSON, form data, etc.)
///
/// # Type Parameters
///
/// * `Output<'r>`: The extracted type, potentially borrowing from the request
/// * `Error`: The error type returned if extraction fails
///
/// # Implementing
///
/// When implementing this trait, consider:
/// - Lifetime requirements of the extracted data
/// - Proper error handling and conversion
/// - Performance implications of extraction
#[async_trait]
pub trait FromRequest {
    /// The type that will be extracted from the request
    type Output<'r>: Send;

    /// The error type returned if extraction fails
    type Error: Responder + Send;

    /// Extracts the type from the request
    ///
    /// # Arguments
    ///
    /// * `req` - The request context containing headers and other metadata
    /// * `body` - Optional request body that can be consumed
    async fn from_request<'r>(req: &'r RequestContext, body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error>;
}

/// Implementation for Option<T> to make extractions optional
#[async_trait]
impl<T> FromRequest for Option<T>
where
    T: FromRequest,
{
    type Output<'r> = Option<T::Output<'r>>;
    type Error = T::Error;

    async fn from_request<'r>(req: &'r RequestContext, body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error> {
        match T::from_request(req, body.clone()).await {
            Ok(t) => Ok(Some(t)),
            Err(_) => Ok(None),
        }
    }
}

#[async_trait]
impl<T> FromRequest for Result<T, T::Error>
where
    T: FromRequest,
{
    type Output<'r> = Result<T::Output<'r>, T::Error>;
    type Error = ParseError;

    async fn from_request<'r>(req: &'r RequestContext, body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error> {
        Ok(T::from_request(req, body).await)
    }
}

/// Implementation for unit type to support handlers without parameters
#[async_trait]
impl FromRequest for () {
    type Output<'r> = ();
    type Error = ParseError;

    async fn from_request(_req: &RequestContext, _body: OptionReqBody) -> Result<Self::Output<'static>, Self::Error> {
        Ok(())
    }
}

/// Responder implementation for ParseError to convert parsing errors to responses
impl Responder for ParseError {
    fn response_to(self, req: &RequestContext) -> Response<ResponseBody> {
        match self {
            ParseError::TooLargeHeader { .. } => {
                (StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE, "payload too large").response_to(req)
            }
            ParseError::TooManyHeaders { .. } => (StatusCode::BAD_REQUEST, "too many headers").response_to(req),
            ParseError::InvalidHeader { .. } => (StatusCode::BAD_REQUEST, "invalid header").response_to(req),
            ParseError::InvalidVersion(_) => (StatusCode::BAD_REQUEST, "invalid version").response_to(req),
            ParseError::InvalidMethod => (StatusCode::BAD_REQUEST, "invalid method").response_to(req),
            ParseError::InvalidUri => (StatusCode::BAD_REQUEST, "invalid uri").response_to(req),
            ParseError::InvalidContentLength { .. } => {
                (StatusCode::BAD_REQUEST, "invalid content length").response_to(req)
            }
            ParseError::InvalidBody { .. } => (StatusCode::BAD_REQUEST, "invalid body").response_to(req),
            ParseError::Io { .. } => (StatusCode::BAD_REQUEST, "connection error").response_to(req),
        }
    }
}
