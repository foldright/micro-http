use micro_http::protocol::body::ReqBody;
use micro_http::protocol::{HttpError, RequestHeader};
use std::future::Future;

pub trait FromRequest: Sized {
    type Error: Into<HttpError>;

    type Future: Future<Output = Result<Self, Self::Error>>;

    fn from_request(req: &RequestHeader, body: &mut ReqBody) -> Self::Future;
}
