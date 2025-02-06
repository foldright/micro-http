use crate::responder::Responder;
use crate::{OptionReqBody, RequestContext, ResponseBody};
use std::convert::Infallible;
use http::{Response, StatusCode};
use micro_http::protocol::ParseError;

#[trait_variant::make(Send)]
pub trait FromRequest {
    type Output<'r>: Send;
    type Error: Responder + Send;

    #[allow(unused)]
    async fn from_request<'r>(
        req: &'r RequestContext<'_, '_>,
        body: OptionReqBody,
    ) -> Result<Self::Output<'r>, Self::Error>;
}

impl<T: FromRequest> FromRequest for Option<T> {
    type Output<'r> = Option<T::Output<'r>>;
    type Error = T::Error;

    async fn from_request<'r>(req: &'r RequestContext<'_, '_>, body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error> {
        match T::from_request(req, body).await {
            Ok(result) => Ok(Some(result)),
            Err(_err) => Ok(None),
        }
    }
}

impl<T: FromRequest> FromRequest for Result<T, T::Error> {
    type Output<'r> = Result<T::Output<'r>, T::Error>;
    type Error = T::Error;

    async fn from_request<'r>(req: &'r RequestContext<'_, '_>, body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error> {
        Ok(T::from_request(req, body).await)
    }
}

impl FromRequest for () {
    type Output<'r> = ();
    type Error = Infallible;

    async fn from_request<'r>(_req: &'r RequestContext<'_, '_>, _body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error> {
        Ok(())
    }
}

/// Responder implementation for ParseError to convert parsing errors to responses
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
