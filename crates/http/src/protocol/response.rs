//! HTTP response header handling implementation.
//!
//! This module provides type definitions for HTTP response headers.
//! It uses the standard `http::Response` type with an empty body placeholder
//! to represent response headers before the actual response body is attached.

use http::Response;

/// Type alias for HTTP response headers.
///
/// This type represents the header portion of an HTTP response, using
/// `http::Response<()>` with an empty body placeholder. The actual response
/// body can be attached later using the response builder pattern.
pub type ResponseHead = Response<()>;
