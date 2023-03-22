use crate::body::ResponseBody;
use crate::fn_trait::FnTrait;
use crate::responder::Responder;
use crate::{FromRequest, OptionReqBody};
use async_trait::async_trait;
use http::{Request, Response};
use micro_http::handler::Handler;
use micro_http::protocol::body::ReqBody;
use micro_http::protocol::RequestHeader;
use std::error::Error;
use std::marker::PhantomData;

/// a `FnTrait` holder which represents any async Fn
pub struct FnHandler<F: FnTrait<Args>, Args> {
    f: F,
    _phantom: PhantomData<fn(Args)>,
}

impl<F, Args> FnHandler<F, Args>
where
    F: FnTrait<Args>,
{
    pub fn new(f: F) -> Self {
        Self { f, _phantom: PhantomData }
    }
}

// TODO 签名还可以精简
#[async_trait]
impl<F, Args> Handler<ReqBody> for FnHandler<F, Args>
where
    F: FnTrait<Args> + for<'r> FnTrait<<Args as FromRequest>::Output<'r>>,
    for<'r> <F as FnTrait<<Args as FromRequest>::Output<'r>>>::Output: Responder,
    Args: FromRequest,
{
    type RespBody = ResponseBody;
    type Error = Box<dyn Error + Send + Sync>;

    async fn call(&self, req: Request<ReqBody>) -> Result<Response<Self::RespBody>, Self::Error> {
        let (parts, body) = req.into_parts();
        let header = RequestHeader::from(parts);
        let req_body = OptionReqBody::from(body);
        let args = Args::from_request(&header, req_body.clone()).await?;
        let responder = self.f.call(args).await;
        Ok(responder.response_to(&header))
    }
}

#[cfg(test)]
mod test {
    use crate::fn_trait::FnTrait;
    use crate::handler::FnHandler;
    use http::Method;
    use micro_http::handler::Handler;
    use micro_http::protocol::body::ReqBody;

    fn assert_is_fn_handler<H: FnTrait<Args>, Args>(_handler: &FnHandler<H, Args>) {
        // no op
    }

    fn assert_is_handler<T: Handler<ReqBody>>(_handler: &T) {
        // no op
    }

    #[test]
    fn assert_fn_is_http_handler_1() {
        async fn get(_header: Method) -> () {
            ()
        }

        let http_handler = FnHandler::new(get);
        assert_is_fn_handler(&http_handler);
        assert_is_handler(&http_handler);
    }

    #[test]
    fn assert_fn_is_http_handler_2() {
        async fn get(_header: &Method, _str: String) -> () {
            ()
        }

        let http_handler = FnHandler::new(get);
        assert_is_fn_handler(&http_handler);
        assert_is_handler(&http_handler);
    }
}
