use crate::body::OptionReqBody;
use crate::extract::from_request::FromRequest;
use http::{HeaderMap, Method};
use micro_http::protocol::{ParseError, RequestHeader};

impl FromRequest for Method {
    type Output<'any> = Method;

    async fn from_request(req: &RequestHeader, _body: OptionReqBody) -> Result<Self::Output<'_>, ParseError> {
        Ok(req.method().clone())
    }
}

impl FromRequest for &Method {
    type Output<'r> = &'r Method;

    async fn from_request(req: &RequestHeader, _body: OptionReqBody) -> Result<Self::Output<'_>, ParseError> {
        Ok(req.method())
    }
}

impl FromRequest for &RequestHeader {
    type Output<'r> = &'r RequestHeader;

    async fn from_request(req: &RequestHeader, _body: OptionReqBody) -> Result<Self::Output<'_>, ParseError> {
        Ok(req)
    }
}

impl FromRequest for &HeaderMap {
    type Output<'r> = &'r HeaderMap;

    async fn from_request(req: &RequestHeader, _body: OptionReqBody) -> Result<Self::Output<'_>, ParseError> {
        Ok(req.headers())
    }
}

impl FromRequest for HeaderMap {
    type Output<'any> = HeaderMap;

    async fn from_request(req: &RequestHeader, _body: OptionReqBody) -> Result<Self::Output<'_>, ParseError> {
        Ok(req.headers().clone())
    }
}
