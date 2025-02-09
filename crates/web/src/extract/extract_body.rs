//! Body data extraction implementations
//!
//! This module provides implementations for extracting typed data from request bodies.
//! It supports extracting raw bytes, strings, JSON data and form data.
//!
//! # Examples
//!
//! ```no_run
//! # use micro_web::extract::{Json, Form};
//! # use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct User {
//!     name: String,
//!     age: u32
//! }
//!
//! // Extract JSON data
//! async fn handle_json(Json(user): Json<User>) {
//!     println!("Got user: {}", user.name);
//! }
//!
//! // Extract form data
//! async fn handle_form(Form(user): Form<User>) {
//!     println!("Got user: {}", user.name);
//! }
//! ```

use crate::body::OptionReqBody;
use crate::extract::from_request::FromRequest;
use crate::extract::{Form, Json};
use crate::RequestContext;
use bytes::Bytes;
use http_body_util::BodyExt;
use micro_http::protocol::ParseError;
use serde::Deserialize;

/// Extracts raw bytes from request body
impl FromRequest for Bytes {
    type Output<'any> = Bytes;
    type Error = ParseError;

    async fn from_request(_req: &RequestContext<'_, '_>, body: OptionReqBody) -> Result<Self::Output<'static>, Self::Error> {
        body.apply(|b| async { b.collect().await.map(|c| c.to_bytes()) }).await
    }
}

/// Extracts UTF-8 string from request body
impl FromRequest for String {
    type Output<'any> = String;
    type Error = ParseError;

    async fn from_request(req: &RequestContext<'_, '_>, body: OptionReqBody) -> Result<Self::Output<'static>, Self::Error> {
        let bytes = <Bytes as FromRequest>::from_request(req, body).await?;
        // todo: using character to decode
        match String::from_utf8(bytes.into()) {
            Ok(s) => Ok(s),
            Err(_) => Err(ParseError::invalid_body("request body is not utf8")),
        }
    }
}

/// Extracts form data from request body
///
/// This implementation expects the request body to be URL-encoded form data
/// and deserializes it into the target type using `serde_urlencoded`.
impl<T> FromRequest for Form<T>
where
    T: for<'de> Deserialize<'de> + Send,
{
    type Output<'r> = Form<T>;
    type Error = ParseError;

    async fn from_request<'r>(req: &'r RequestContext<'_, '_>, body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error> {
        let bytes = <Bytes as FromRequest>::from_request(req, body).await?;
        serde_urlencoded::from_bytes::<'_, T>(&bytes).map(|t| Form(t)).map_err(|e| ParseError::invalid_body(e.to_string()))
    }
}

/// Extracts JSON data from request body
///
/// This implementation expects the request body to be valid JSON
/// and deserializes it into the target type using `serde_json`.
impl<T> FromRequest for Json<T>
where
    T: for<'de> Deserialize<'de> + Send,
{
    type Output<'r> = Json<T>;
    type Error = ParseError;

    async fn from_request<'r>(req: &'r RequestContext<'_, '_>, body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error> {
        let bytes = <Bytes as FromRequest>::from_request(req, body).await?;
        serde_json::from_slice::<'_, T>(&bytes).map(|t| Json(t)).map_err(|e| ParseError::invalid_body(e.to_string()))
    }
}
