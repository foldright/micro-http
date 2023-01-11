use std::error::Error;
use std::fmt::{Display, Formatter};
use std::str::Utf8Error;

#[derive(Debug, PartialEq)]
pub enum ParseError {
    InvalidEncoding,
    InvalidRequest,
    InvalidVersion,
}


impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidEncoding => write!(f, "invalid encoding"),
            Self::InvalidRequest => write!(f, "invalid request"),
            Self::InvalidVersion => write!(f, "invalid version"),
        }
    }
}

impl Error for ParseError {}

impl From<Utf8Error> for ParseError {
    fn from(_: Utf8Error) -> Self {
        Self::InvalidEncoding
    }
}