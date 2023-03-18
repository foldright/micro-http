use crate::body::OptionReqBody;
use crate::extract::from_request::FromRequest;
use http::{HeaderMap, Method};
use micro_http::protocol::{ParseError, RequestHeader};

impl<'r> FromRequest<'r> for Method {
    type Output = Method;

    async fn from_request(req: &'r RequestHeader, _body: OptionReqBody) -> Result<Self::Output, ParseError> {
        Ok(req.method().clone())
    }
}

impl<'r> FromRequest<'r> for &Method {
    type Output = &'r Method;

    async fn from_request(req: &'r RequestHeader, _body: OptionReqBody) -> Result<Self::Output, ParseError> {
        Ok(req.method())
    }
}

impl<'r> FromRequest<'r> for &RequestHeader {
    type Output = &'r RequestHeader;

    async fn from_request(req: &'r RequestHeader, _body: OptionReqBody) -> Result<Self::Output, ParseError> {
        Ok(req)
    }
}

impl<'r> FromRequest<'r> for &HeaderMap {
    type Output = &'r HeaderMap;

    async fn from_request(req: &'r RequestHeader, _body: OptionReqBody) -> Result<Self::Output, ParseError> {
        Ok(req.headers())
    }
}

impl<'r> FromRequest<'r> for HeaderMap {
    type Output = HeaderMap;

    async fn from_request(req: &'r RequestHeader, _body: OptionReqBody) -> Result<Self::Output, ParseError> {
        Ok(req.headers().clone())
    }
}
