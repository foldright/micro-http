use crate::body::ResponseBody;
use crate::RequestContext;
use http::{Response, StatusCode};

pub trait Responder {
    fn response_to(self, req: &RequestContext) -> Response<ResponseBody>;
}

impl<T: Responder, E: Responder> Responder for Result<T, E> {
    fn response_to(self, req: &RequestContext) -> Response<ResponseBody> {
        match self {
            Ok(t) => t.response_to(req),
            Err(e) => e.response_to(req),
        }
    }
}

impl<T: Responder> Responder for Option<T> {
    fn response_to(self, req: &RequestContext) -> Response<ResponseBody> {
        match self {
            Some(t) => t.response_to(req),
            None => Response::new(ResponseBody::empty()),
        }
    }
}

impl<B> Responder for Response<B>
where
    B: Into<ResponseBody>,
{
    fn response_to(self, _req: &RequestContext) -> Response<ResponseBody> {
        self.map(|b| b.into())
    }
}

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
        let mut builder = Response::builder();
        let headers = builder.headers_mut().unwrap();
        headers.reserve(8);
        headers.insert(http::header::CONTENT_TYPE, mime::TEXT_PLAIN_UTF_8.as_ref().parse().unwrap());

        builder.status(StatusCode::OK).body(ResponseBody::from(self)).unwrap()
    }
}

impl Responder for String {
    fn response_to(self, _req: &RequestContext) -> Response<ResponseBody> {
        let mut builder = Response::builder();
        let headers = builder.headers_mut().unwrap();
        headers.reserve(8);
        headers.insert(http::header::CONTENT_TYPE, mime::TEXT_PLAIN_UTF_8.as_ref().parse().unwrap());

        builder.status(StatusCode::OK).body(ResponseBody::from(self)).unwrap()
    }
}
