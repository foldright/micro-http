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

    #[error("invalid content-length header: {reason}")]
    InvalidContentLength { reason: String },

    #[error("invalid body: {reason}")]
    InvalidBody { reason: String },

    #[error("io error: {source}")]
    Io { #[from] source: io::Error},
}

#[derive(Error, Debug)]
pub enum SendError {
    #[error("invalid body: {reason}")]
    InvalidBody {reason: String},

    #[error("io error: {source}")]
    Io { #[from] source: io::Error},
}