use crate::body::ResponseBody;
use http::{Response, StatusCode};
use micro_http::protocol::RequestHeader;

pub trait Responder {
    fn response_to(self, req: &RequestHeader) -> Response<ResponseBody>;
}

// todo: impl for Result & Option

impl<T: Responder> Responder for (T, StatusCode) {
    fn response_to(self, req: &RequestHeader) -> Response<ResponseBody> {
        let (responder, status) = self;
        let mut response = responder.response_to(req);
        *response.status_mut() = status;
        response
    }
}

impl<T: Responder> Responder for Box<T> {
    fn response_to(self, req: &RequestHeader) -> Response<ResponseBody> {
        (*self).response_to(req)
    }
}

impl Responder for () {
    fn response_to(self, _req: &RequestHeader) -> Response<ResponseBody> {
        Response::new(ResponseBody::empty())
    }
}

impl Responder for &'static str {
    fn response_to(self, _req: &RequestHeader) -> Response<ResponseBody> {
        Response::builder().body(ResponseBody::from(self)).unwrap()
    }
}

impl Responder for String {
    fn response_to(self, _req: &RequestHeader) -> Response<ResponseBody> {
        Response::builder()
            .status(StatusCode::OK)
            .header(http::header::CONTENT_LENGTH, self.len())
            .body(ResponseBody::from(self))
            .unwrap()
    }
}
