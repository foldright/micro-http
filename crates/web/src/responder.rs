use crate::body::RespBody;
use http::{Response, StatusCode};
use micro_http::protocol::RequestHeader;

pub trait Responder {
    fn response_to(self, req: &RequestHeader) -> Response<RespBody>;
}

// todo: impl for Result & Option

impl<T: Responder> Responder for (T, StatusCode) {
    fn response_to(self, req: &RequestHeader) -> Response<RespBody> {
        let (responder, status) = self;
        let mut response = responder.response_to(req);
        *response.status_mut() = status;
        response
    }
}

impl<T: Responder> Responder for Box<T> {
    fn response_to(self, req: &RequestHeader) -> Response<RespBody> {
        (*self).response_to(req)
    }
}

impl Responder for () {
    fn response_to(self, _req: &RequestHeader) -> Response<RespBody> {
        Response::new(RespBody::empty())
    }
}

impl Responder for &'static str {
    fn response_to(self, _req: &RequestHeader) -> Response<RespBody> {
        Response::builder().body(RespBody::from(self)).unwrap()
    }
}

impl Responder for String {
    fn response_to(self, _req: &RequestHeader) -> Response<RespBody> {
        Response::builder()
            .status(StatusCode::OK)
            .header(http::header::CONTENT_LENGTH, self.len())
            .body(RespBody::from(self))
            .unwrap()
    }
}
