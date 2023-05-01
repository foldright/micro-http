use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpError {
    #[error("request error: {source}")]
    RequestError {
        #[from]
        source: ParseError,
    },

    #[error("response error: {source}")]
    ResponseError {
        #[from]
        source: SendError,
    },
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("header size too large, current: {current_size} exceed the limit {max_size}")]
    TooLargeHeader { current_size: usize, max_size: usize },

    #[error("header number exceed the limit {max_num}")]
    TooManyHeaders { max_num: usize },

    #[error("invalid header: {reason}")]
    InvalidHeader { reason: String },

    #[error("invalid http version: {0:?}")]
    InvalidVersion(Option<u8>),

    #[error("invalid http method")]
    InvalidMethod,

    #[error("invalid http uri")]
    InvalidUri,

    #[error("invalid content-length header: {reason}")]
    InvalidContentLength { reason: String },

    #[error("invalid body: {reason}")]
    InvalidBody { reason: String },

    #[error("io error: {source}")]
    Io {
        #[from]
        source: io::Error,
    },
}

impl ParseError {
    pub fn too_large_header(current_size: usize, max_size: usize) -> Self {
        Self::TooLargeHeader { current_size, max_size }
    }

    pub fn too_many_headers(max_num: usize) -> Self {
        Self::TooManyHeaders { max_num }
    }

    pub fn invalid_header<S: ToString>(str: S) -> Self {
        Self::InvalidHeader { reason: str.to_string() }
    }

    pub fn invalid_body<S: ToString>(str: S) -> Self {
        Self::InvalidBody { reason: str.to_string() }
    }

    pub fn invalid_content_length<S: ToString>(str: S) -> Self {
        Self::InvalidContentLength { reason: str.to_string() }
    }

    pub fn io<E: Into<io::Error>>(e: E) -> Self {
        Self::Io { source: e.into() }
    }
}

#[derive(Error, Debug)]
pub enum SendError {
    #[error("invalid body: {reason}")]
    InvalidBody { reason: String },

    #[error("io error: {source}")]
    Io {
        #[from]
        source: io::Error,
    },
}

impl SendError {
    pub fn invalid_body<S: ToString>(str: S) -> Self {
        Self::InvalidBody { reason: str.to_string() }
    }

    pub fn io<E: Into<io::Error>>(e: E) -> Self {
        Self::Io { source: e.into() }
    }
}
