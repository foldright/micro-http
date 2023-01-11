use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io;
use std::string::ParseError;

#[derive(Debug, PartialEq)]
pub enum ServerError {
    BrokenConnection,
    BufSizeFull,
    IoError(io::ErrorKind),
    RequestError(ParseError),
}

impl Display for ServerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BrokenConnection => write!(f, "broken connection"),
            Self::BufSizeFull => write!(f, "read buffer size reached"),
            Self::IoError(e) => write!(f, "read buffer size reached {}", e),
            Self::RequestError(e) => write!(f, "parse http protocol error {}", e),
        }
    }
}

impl Error for ServerError {}