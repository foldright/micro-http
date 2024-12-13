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

use crate::extract::{FromRequest, Query};
use crate::{OptionReqBody, RequestContext};
use async_trait::async_trait;
use micro_http::protocol::ParseError;
use serde::Deserialize;

/// Implements query string extraction for any type that implements Deserialize
/// 
/// This implementation allows automatic deserialization of query string parameters
/// into a strongly-typed struct using serde_qs.
#[async_trait]
impl<T> FromRequest for Query<T>
where
    T: for<'de> Deserialize<'de> + Send,
{
    type Output<'r> = T;
    type Error = ParseError;

    async fn from_request<'r>(req: &'r RequestContext, _body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error> {
        let query = req.uri().query().ok_or_else(|| ParseError::invalid_header("has no query string, path"))?;
        serde_qs::from_str::<'_, T>(query).map_err(|e| ParseError::invalid_header(e.to_string()))
    }
}
