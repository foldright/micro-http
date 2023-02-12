use snafu::prelude::*;

use std::io::Error as IoError;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum DecodeError {
    #[snafu(display("header too large, current size: {}, max size: {max_size}"))]
    TooLargeHeader { current_size: usize, max_size: usize },

    #[snafu(display("header too many, max num: {max_num}"))]
    TooManyHeaders { max_num: usize, source: httparse::Error },

    #[snafu(display("parsed invalid header: {source}"))]
    InvalidHeader { source: httparse::Error },

    #[snafu(display("io error happens when parsing : {source}"), context(false))]
    Io { source: IoError },

    #[snafu(display("invalid content-length: {message}"))]
    InvalidContentLength { message: String },

    #[snafu(display("parse body error: {message}"))]
    Body { message: String },
}
