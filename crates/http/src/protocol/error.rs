//! Error types for HTTP protocol handling
//!
//! This module provides error types for handling various error conditions that may occur
//! during HTTP request processing and response generation.
//!
//! # Error Types
//!
//! - [`HttpError`]: The top-level error type that wraps all other error types
//!   - [`ParseError`]: Errors that occur during request parsing and processing
//!   - [`SendError`]: Errors that occur during response generation and sending
//!
//! The error types form a hierarchy where `HttpError` is the top-level error that can
//! contain either a `ParseError` or `SendError`. This allows for granular error handling
//! while still providing a unified error type at the API boundary.
use std::io;
use thiserror::Error;

/// The top-level error type for HTTP operations
///
/// This enum represents all possible errors that can occur during HTTP request
/// processing and response generation.
#[derive(Debug, Error)]
pub enum HttpError {
    /// Errors that occur during request parsing and processing
    #[error("request error: {source}")]
    RequestError {
        #[from]
        source: ParseError,
    },

    /// Errors that occur during response generation and sending
    #[error("response error: {source}")]
    ResponseError {
        #[from]
        source: SendError,
    },
}

/// Errors that occur during HTTP request parsing
///
/// This enum represents various error conditions that can occur while parsing
/// and processing HTTP requests.
#[derive(Error, Debug)]
pub enum ParseError {
    /// Header size exceeds the maximum allowed size
    #[error("header size too large, current: {current_size} exceed the limit {max_size}")]
    TooLargeHeader { current_size: usize, max_size: usize },

    /// Number of headers exceeds the maximum allowed
    #[error("header number exceed the limit {max_num}")]
    TooManyHeaders { max_num: usize },

    /// Invalid header format or content
    #[error("invalid header: {reason}")]
    InvalidHeader { reason: String },

    /// Unsupported HTTP version
    #[error("invalid http version: {0:?}")]
    InvalidVersion(Option<u8>),

    /// Invalid or unsupported HTTP method
    #[error("invalid http method")]
    InvalidMethod,

    /// Invalid URI format
    #[error("invalid http uri")]
    InvalidUri,

    /// Invalid Content-Length header
    #[error("invalid content-length header: {reason}")]
    InvalidContentLength { reason: String },

    /// Invalid request body
    #[error("invalid body: {reason}")]
    InvalidBody { reason: String },

    /// I/O error during parsing
    #[error("io error: {source}")]
    Io {
        #[from]
        source: io::Error,
    },
}

impl ParseError {
    /// Creates a new TooLargeHeader error
    pub fn too_large_header(current_size: usize, max_size: usize) -> Self {
        Self::TooLargeHeader { current_size, max_size }
    }

    /// Creates a new TooManyHeaders error
    pub fn too_many_headers(max_num: usize) -> Self {
        Self::TooManyHeaders { max_num }
    }

    /// Creates a new InvalidHeader error
    pub fn invalid_header<S: ToString>(str: S) -> Self {
        Self::InvalidHeader { reason: str.to_string() }
    }

    /// Creates a new InvalidBody error
    pub fn invalid_body<S: ToString>(str: S) -> Self {
        Self::InvalidBody { reason: str.to_string() }
    }

    /// Creates a new InvalidContentLength error
    pub fn invalid_content_length<S: ToString>(str: S) -> Self {
        Self::InvalidContentLength { reason: str.to_string() }
    }

    /// Creates a new I/O error
    pub fn io<E: Into<io::Error>>(e: E) -> Self {
        Self::Io { source: e.into() }
    }
}

/// Errors that occur during HTTP response generation and sending
///
/// This enum represents error conditions that can occur while generating
/// and sending HTTP responses.
#[derive(Error, Debug)]
pub enum SendError {
    /// Invalid response body
    #[error("invalid body: {reason}")]
    InvalidBody { reason: String },

    /// I/O error during sending
    #[error("io error: {source}")]
    Io {
        #[from]
        source: io::Error,
    },
}

impl SendError {
    /// Creates a new InvalidBody error
    pub fn invalid_body<S: ToString>(str: S) -> Self {
        Self::InvalidBody { reason: str.to_string() }
    }

    /// Creates a new I/O error
    pub fn io<E: Into<io::Error>>(e: E) -> Self {
        Self::Io { source: e.into() }
    }
}
