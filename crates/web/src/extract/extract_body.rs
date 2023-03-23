use crate::body::OptionReqBody;
use crate::{FromRequest, RequestContext};
use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::BodyExt;
use micro_http::protocol::ParseError;

#[async_trait]
impl FromRequest for Bytes {
    type Output<'any> = Bytes;

    async fn from_request(_req: &RequestContext, body: OptionReqBody) -> Result<Self::Output<'static>, ParseError> {
        body.apply(|b| async { b.collect().await.map(|c| c.to_bytes()) }).await
    }
}

#[async_trait]
impl FromRequest for String {
    type Output<'any> = String;

    async fn from_request(req: &RequestContext, body: OptionReqBody) -> Result<Self::Output<'static>, ParseError> {
        let bytes = Bytes::from_request(req, body).await?;
        // todo: using character to decode
        match String::from_utf8(bytes.into()) {
            Ok(s) => Ok(s),
            Err(_) => Err(ParseError::invalid_body("request body is not utf8")),
        }
    }
}
