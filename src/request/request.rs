use std::fmt::{Debug, Display, Formatter};
use std::str::Utf8Error;
use crate::request::error::ParseError;
use crate::request::method::Method;
use crate::request::query::Query;

pub struct Request<'a> {
    method: Method,
    path: &'a str,
    query: Query<'a>,
    http_version: HttpVersion,
    headers: Headers,
}

pub enum HttpVersion {
    Http1_1
}

pub struct Headers {}

impl<'a> TryFrom<&'a [u8]> for Request<'a> {
    type Error = ParseError;

    fn try_from(str: &[u8]) -> Result<Self, Self::Error> {
        let str = std::str::from_utf8(str)?;

        let (method_str, remaining) = str.split_once(' ')
            .ok_or(ParseError::InvalidRequest)?;

        let method = Method::try_from(method_str)?;

        let (path_query, remaining) = remaining.split_once(' ')
            .ok_or(ParseError::InvalidRequest)?;

        let (mut path, mut query) = (path_query, "");
        if let Some(split) = path_query.split_once('?') {
            (path, query) = split;
        }


        todo!()
    }
}

// GET /index?a=1&c=2&b=3 HTTP/1.1
// Host: 127.0.0.1:8080
// User-Agent: curl/7.79.1
// Accept: */*




