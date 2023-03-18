use crate::body::OptionReqBody;
use crate::FromRequest;
use bytes::Bytes;
use http_body_util::BodyExt;
use micro_http::protocol::{ParseError, RequestHeader};

impl<'r> FromRequest<'r> for Bytes {
    type Output = Bytes;

    async fn from_request(_req: &'r RequestHeader, body: OptionReqBody) -> Result<Self::Output, ParseError> {
        body.apply(|b| async { b.collect().await.map(|c| c.to_bytes()) }).await
    }
}

impl<'r> FromRequest<'r> for String {
    type Output = String;

    async fn from_request(req: &'r RequestHeader, body: OptionReqBody) -> Result<Self::Output, ParseError> {
        let bytes = Bytes::from_request(req, body).await?;
        // todo: using character to decode
        match String::from_utf8(bytes.into()) {
            Ok(s) => Ok(s),
            Err(_) => Err(ParseError::invalid_body("request body is not utf8")),
        }
    }
}
