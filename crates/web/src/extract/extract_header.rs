use async_trait::async_trait;
use crate::body::OptionReqBody;
use crate::extract::from_request::FromRequest;
use http::{HeaderMap, Method};
use micro_http::protocol::{ParseError, RequestHeader};
use crate::RequestContext;

#[async_trait]
impl FromRequest for Method {
    type Output<'any> = Method;

    async fn from_request(req: &RequestContext, _body: OptionReqBody) -> Result<Self::Output<'static>, ParseError> {
        Ok(req.method().clone())
    }
}

#[async_trait]
impl FromRequest for &Method {
    type Output<'r> = &'r Method;

    async fn from_request<'r>(req: &'r RequestContext, _body: OptionReqBody) -> Result<Self::Output<'r>, ParseError> {
        Ok(req.method())
    }
}

#[async_trait]
impl FromRequest for &RequestHeader {
    type Output<'r> = &'r RequestHeader;

    async fn from_request<'r>(req: &'r RequestContext, _body: OptionReqBody) -> Result<Self::Output<'r>, ParseError> {
        Ok(req.request_header())
    }
}

#[async_trait]
impl FromRequest for &HeaderMap {
    type Output<'r> = &'r HeaderMap;

    async fn from_request<'r>(req: &'r RequestContext, _body: OptionReqBody) -> Result<Self::Output<'r>, ParseError> {
        Ok(req.headers())
    }
}

#[async_trait]
impl FromRequest for HeaderMap {
    type Output<'any> = HeaderMap;

    async fn from_request(req: &RequestContext, _body: OptionReqBody) -> Result<Self::Output<'static>, ParseError> {
        Ok(req.headers().clone())
    }
}
