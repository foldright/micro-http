use crate::body::OptionReqBody;
use crate::responder::Responder;
use crate::{RequestContext, ResponseBody};
use async_trait::async_trait;
use http::{Response, StatusCode};
use micro_http::protocol::ParseError;

#[async_trait]
pub trait FromRequest {
    type Output<'r>: Send;
    type Error: Responder + Send;
    async fn from_request<'r>(req: &'r RequestContext, body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error>;
}

#[async_trait]
impl<T> FromRequest for Option<T>
where
    T: FromRequest,
{
    type Output<'r> = Option<T::Output<'r>>;
    type Error = T::Error;

    async fn from_request<'r>(req: &'r RequestContext, body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error> {
        match T::from_request(req, body.clone()).await {
            Ok(t) => Ok(Some(t)),
            Err(_) => Ok(None),
        }
    }
}

#[async_trait]
impl<T> FromRequest for Result<T, T::Error>
where
    T: FromRequest,
{
    type Output<'r> = Result<T::Output<'r>, T::Error>;
    type Error = ParseError;

    async fn from_request<'r>(req: &'r RequestContext, body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error> {
        Ok(T::from_request(req, body).await)
    }
}

#[async_trait]
impl FromRequest for () {
    type Output<'r> = ();
    type Error = ParseError;

    async fn from_request(_req: &RequestContext, _body: OptionReqBody) -> Result<Self::Output<'static>, Self::Error> {
        Ok(())
    }
}

impl Responder for ParseError {
    fn response_to(self, req: &RequestContext) -> Response<ResponseBody> {
        match self {
            ParseError::TooLargeHeader { .. } => {
                (StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE, "payload too large").response_to(req)
            }
            ParseError::TooManyHeaders { .. } => (StatusCode::BAD_REQUEST, "too many headers").response_to(req),
            ParseError::InvalidHeader { .. } => (StatusCode::BAD_REQUEST, "invalid header").response_to(req),
            ParseError::InvalidVersion(_) => (StatusCode::BAD_REQUEST, "invalid version").response_to(req),
            ParseError::InvalidMethod => (StatusCode::BAD_REQUEST, "invalid method").response_to(req),
            ParseError::InvalidUri => (StatusCode::BAD_REQUEST, "invalid uri").response_to(req),
            ParseError::InvalidContentLength { .. } => {
                (StatusCode::BAD_REQUEST, "invalid content length").response_to(req)
            }
            ParseError::InvalidBody { .. } => (StatusCode::BAD_REQUEST, "invalid body").response_to(req),
            ParseError::Io { .. } => (StatusCode::BAD_REQUEST, "connection error").response_to(req),

        }
    }
}
