use std::fmt::{Display, Formatter};
use std::io::Write;
use crate::protocol::http_version::HttpVersion;

pub struct Response {
    http_version: HttpVersion,
    status_code: StatusCode,
    // headers: Headers
    body: Option<String>,
}

#[derive(Copy, Clone)]
pub enum StatusCode {
    OK = 200,
    //...
}

impl Display for StatusCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u16)
    }
}

impl StatusCode {
    fn phase(&self) -> &str {
        match self {
            Self::OK => "OK",
        }
    }
}

impl Response {
    pub fn new(status_code: StatusCode, body: Option<String>) -> Self {
        Self {
            http_version: HttpVersion::Http1_1,
            status_code,
            body,
        }
    }


    pub fn write(&self, writer: &mut impl Write) {
        let mut response_str = format!("{} {} {}\r\n\r\n",
                                       self.http_version,
                                       self.status_code,
                                       self.status_code.phase());

        if let Some(content) = &self.body {
            response_str = response_str + content;
        }

        writer.write(response_str.as_bytes()).unwrap();
    }
}