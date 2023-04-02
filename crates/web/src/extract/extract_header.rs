use async_trait::async_trait;
use crate::body::OptionReqBody;
use crate::extract::from_request::FromRequest;
use http::{HeaderMap, Method};
use micro_http::protocol::{ParseError, RequestHeader};
use crate::RequestContext;

#[async_trait]
impl FromRequest for Method {
    type Output<'any> = Method;
    type Error = ParseError;

    async fn from_request(req: &RequestContext, _body: OptionReqBody) -> Result<Self::Output<'static>, Self::Error> {
        Ok(req.method().clone())
    }
}

#[async_trait]
impl FromRequest for &Method {
    type Output<'r> = &'r Method;
    type Error = ParseError;

    async fn from_request<'r>(req: &'r RequestContext, _body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error> {
        Ok(req.method())
    }
}

#[async_trait]
impl FromRequest for &RequestHeader {
    type Output<'r> = &'r RequestHeader;
    type Error = ParseError;

    async fn from_request<'r>(req: &'r RequestContext, _body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error> {
        Ok(req.request_header())
    }
}

#[async_trait]
impl FromRequest for &HeaderMap {
    type Output<'r> = &'r HeaderMap;
    type Error = ParseError;

    async fn from_request<'r>(req: &'r RequestContext, _body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error> {
        Ok(req.headers())
    }
}

#[async_trait]
impl FromRequest for HeaderMap {
    type Output<'any> = HeaderMap;
    type Error = ParseError;

    async fn from_request(req: &RequestContext, _body: OptionReqBody) -> Result<Self::Output<'static>, Self::Error> {
        Ok(req.headers().clone())
    }
}
