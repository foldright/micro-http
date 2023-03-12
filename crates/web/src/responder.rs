use bytes::Bytes;
use http::{Response, StatusCode};
use http_body::Body;
use http_body_util::{Empty, Full};
use micro_http::protocol::RequestHeader;

pub trait Responder {
    type Body: Body;

    fn response_to(self, req: &RequestHeader) -> Response<Self::Body>;
}

// todo: impl for Result & Option

impl<T: Responder> Responder for (T, StatusCode) {
    type Body = T::Body;

    fn response_to(self, req: &RequestHeader) -> Response<Self::Body> {
        let (responder, status) = self;
        let mut response = responder.response_to(req);
        *response.status_mut() = status;
        response
    }
}

impl<T: Responder> Responder for Box<T> {
    type Body = T::Body;

    fn response_to(self, req: &RequestHeader) -> Response<Self::Body> {
        (*self).response_to(req)
    }
}

impl Responder for () {
    type Body = Empty<Bytes>;

    fn response_to(self, _req: &RequestHeader) -> Response<Self::Body> {
        Response::new(Empty::new())
    }
}

impl Responder for &'static str {
    type Body = Full<&'static [u8]>;

    fn response_to(self, _req: &RequestHeader) -> Response<Self::Body> {
        Response::builder().body(Full::new(self.as_bytes())).unwrap()
    }
}

impl Responder for String {
    type Body = String;

    fn response_to(self, _req: &RequestHeader) -> Response<Self::Body> {
        Response::builder()
            .status(StatusCode::OK)
            .header(http::header::CONTENT_LENGTH, self.len())
            .body(self)
            .unwrap()
    }
}
