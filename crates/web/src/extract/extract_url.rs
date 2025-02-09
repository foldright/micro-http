//! URL query string extraction functionality
//!
//! This module provides implementation for extracting typed data from URL query strings.
//! It allows handlers to receive strongly-typed query parameters by implementing the
//! `FromRequest` trait for the `Query<T>` type.
//!
//! # Example
//! ```no_run
//! # use serde::Deserialize;
//! # use micro_web::extract::Query;
//!
//! #[derive(Deserialize)]
//! struct Params {
//!     name: String,
//!     age: u32,
//! }
//!
//! async fn handler(Query(params): Query<Params>) {
//!     println!("Name: {}, Age: {}", params.name, params.age);
//! }
//! ```

use crate::extract::from_request::FromRequest;
use crate::extract::Query;
use crate::{OptionReqBody, PathParams, RequestContext};
use micro_http::protocol::ParseError;
use serde::Deserialize;

/// Implements query string extraction for any type that implements Deserialize
///
/// This implementation allows automatic deserialization of query string parameters
/// into a strongly-typed struct using serde_qs.
impl<T> FromRequest for Query<T>
where
    T: for<'de> Deserialize<'de> + Send,
{
    type Output<'r> = T;
    type Error = ParseError;

    async fn from_request<'r>(req: &'r RequestContext<'_, '_>, _body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error> {
        let query = req.uri().query().ok_or_else(|| ParseError::invalid_header("has no query string, path"))?;
        serde_qs::from_str::<'_, T>(query).map_err(|e| ParseError::invalid_header(e.to_string()))
    }
}

/// Implements path parameter extraction for referenced PathParams
///
/// This implementation is similar to the owned version but works with references
/// to PathParams. It allows handlers to receive path parameters as references
/// directly from the request context.
impl FromRequest for &PathParams<'_, '_> {
    type Output<'r> = &'r PathParams<'r, 'r>;
    type Error = ParseError;

    async fn from_request<'r>(req: &'r RequestContext<'_, '_>, _body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error> {
        Ok(req.path_params())
    }
}
