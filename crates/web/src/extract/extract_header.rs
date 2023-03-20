use async_trait::async_trait;
use crate::body::OptionReqBody;
use crate::extract::from_request::FromRequest;
use http::{HeaderMap, Method};
use micro_http::protocol::{ParseError, RequestHeader};

#[async_trait]
impl FromRequest for Method {
    type Output<'any> = Method;

    async fn from_request(req: &RequestHeader, _body: OptionReqBody) -> Result<Self::Output<'_>, ParseError> {
        Ok(req.method().clone())
    }
}

#[async_trait]
impl FromRequest for &Method {
    type Output<'r> = &'r Method;

    async fn from_request(req: &RequestHeader, _body: OptionReqBody) -> Result<Self::Output<'_>, ParseError> {
        Ok(req.method())
    }
}

#[async_trait]
impl FromRequest for &RequestHeader {
    type Output<'r> = &'r RequestHeader;

    async fn from_request(req: &RequestHeader, _body: OptionReqBody) -> Result<Self::Output<'_>, ParseError> {
        Ok(req)
    }
}

#[async_trait]
impl FromRequest for &HeaderMap {
    type Output<'r> = &'r HeaderMap;

    async fn from_request(req: &RequestHeader, _body: OptionReqBody) -> Result<Self::Output<'_>, ParseError> {
        Ok(req.headers())
    }
}

#[async_trait]
impl FromRequest for HeaderMap {
    type Output<'any> = HeaderMap;

    async fn from_request(req: &RequestHeader, _body: OptionReqBody) -> Result<Self::Output<'_>, ParseError> {
        Ok(req.headers().clone())
    }
}
