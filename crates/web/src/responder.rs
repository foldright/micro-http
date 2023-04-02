use crate::body::ResponseBody;
use crate::RequestContext;
use http::{Response, StatusCode};

pub trait Responder {
    fn response_to(self, req: &RequestContext) -> Response<ResponseBody>;
}

// todo: impl for Result & Option

impl<T: Responder> Responder for (StatusCode, T) {
    fn response_to(self, req: &RequestContext) -> Response<ResponseBody> {
        let (status, responder) = self;
        let mut response = responder.response_to(req);
        *response.status_mut() = status;
        response
    }
}

impl<T: Responder> Responder for (T, StatusCode) {
    fn response_to(self, req: &RequestContext) -> Response<ResponseBody> {
        let (responder, status) = self;
        (status, responder).response_to(req)
    }
}

impl<T: Responder> Responder for Box<T> {
    fn response_to(self, req: &RequestContext) -> Response<ResponseBody> {
        (*self).response_to(req)
    }
}

impl Responder for () {
    fn response_to(self, _req: &RequestContext) -> Response<ResponseBody> {
        Response::new(ResponseBody::empty())
    }
}

impl Responder for &'static str {
    fn response_to(self, _req: &RequestContext) -> Response<ResponseBody> {
        Response::builder().body(ResponseBody::from(self)).unwrap()
    }
}

impl Responder for String {
    fn response_to(self, _req: &RequestContext) -> Response<ResponseBody> {
        Response::builder().status(StatusCode::OK).body(ResponseBody::from(self)).unwrap()
    }
}
