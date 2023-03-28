use crate::{OptionReqBody, RequestContext, ResponseBody};
use async_trait::async_trait;
use http::Response;
use std::error::Error;

pub type ResponseResult = Result<Response<ResponseBody>, Box<dyn Error + Send + Sync>>;

#[async_trait]
pub trait Interceptor: Send + Sync {
    async fn on_request(&self, _req: &mut RequestContext, _body: &mut OptionReqBody) {}

    async fn on_response(&self, _req: &RequestContext, _resp: &mut ResponseResult) {}
}

pub struct Interceptors {
    inner: Vec<Box<dyn Interceptor>>,
}

#[async_trait]
impl Interceptor for Interceptors {
    async fn on_request(&self, req: &mut RequestContext, body: &mut OptionReqBody) {
        for interceptor in self.inner.iter() {
            interceptor.on_request(req, body).await;
        }
    }

    async fn on_response(&self, req: &RequestContext, resp: &mut ResponseResult) {
        for interceptor in self.inner.iter() {
            interceptor.on_response(req, resp).await;
        }
    }
}

impl Interceptors {
    pub fn builder() -> InterceptorsBuilder {
        InterceptorsBuilder::new()
    }
}

pub struct InterceptorsBuilder {
    inner: Vec<Box<dyn Interceptor>>,
}

impl InterceptorsBuilder {
    fn new() -> Self {
        Self { inner: vec![] }
    }

    pub fn add_last<I: Interceptor + Send + Sync + 'static>(mut self, interceptor: I) -> Self {
        self.inner.push(Box::new(interceptor));
        self
    }

    pub fn add_first<I: Interceptor + Send + Sync + 'static>(mut self, interceptor: I) -> Self {
        self.inner.insert(0, Box::new(interceptor));
        self
    }

    pub fn build(self) -> Interceptors {
        Interceptors { inner: self.inner }
    }
}
