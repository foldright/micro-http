use crate::extract::{FromRequest, Query};
use crate::{OptionReqBody, RequestContext};
use async_trait::async_trait;
use micro_http::protocol::ParseError;
use serde::Deserialize;

#[async_trait]
impl<T> FromRequest for Query<T>
where
    T: for<'de> Deserialize<'de> + Send,
{
    type Output<'r> = T;
    type Error = ParseError;

    async fn from_request<'r>(req: &'r RequestContext, _body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error> {
        let query = req.uri().query().ok_or_else(|| ParseError::invalid_header("has no query string, path"))?;
        serde_qs::from_str::<'_, T>(query).map_err(|e| ParseError::invalid_header(e.to_string()))
    }
}
