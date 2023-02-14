use std::error::Error;

use crate::protocol::body::ReqBody;
use async_trait::async_trait;
use http::{Request, Response};

use http_body::Body;

#[async_trait]
pub trait Handler: Send + Sync + 'static {
    type RespBody: Body;

    type Error: Into<Box<dyn Error + Send>>;

    async fn handle(&self, request: Request<ReqBody>) -> Result<Response<Self::RespBody>, Self::Error>;
}