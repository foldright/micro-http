use crate::fn_trait::FnTrait;
use crate::responder::Responder;
use crate::FromRequest;
use http::{Request, Response};
use http_body::Body;
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

impl<F, Args, RespBody> Handler<ReqBody> for FnHandler<F, Args>
where
    F: FnTrait<Args> + for<'r> FnTrait<<Args as FromRequest<'r>>::Output>,
    for<'r> <F as FnTrait<<Args as FromRequest<'r>>::Output>>::Output: Responder<Body = RespBody>,
    RespBody: Body,
    Args: for<'r> FromRequest<'r>,
{
    type RespBody = RespBody;
    type Error = Box<dyn Error + Send + Sync>;

    async fn call(&self, req: Request<ReqBody>) -> Result<Response<Self::RespBody>, Self::Error> {
        let (parts, mut body) = req.into_parts();
        let header = RequestHeader::from(parts);
        let args = Args::from_request(&header).await?;
        let responder = self.f.call(args).await;
        Ok(responder.response_to(&header))
    }
}

#[cfg(test)]
mod test {
    use crate::fn_trait::FnTrait;
    use crate::handler::FnHandler;
    use crate::responder::Responder;
    use bytes::Bytes;
    use http::{Method, Response};
    use http_body_util::Empty;
    use micro_http::handler::Handler;
    use micro_http::protocol::body::ReqBody;
    use micro_http::protocol::RequestHeader;
    use std::error::Error;
    use std::marker::PhantomData;

    fn assert_is_fn_handler<H: FnTrait<Args>, Args>(_handler: &FnHandler<H, Args>) {
        // no op
    }

    fn assert_is_handler<T: Handler<ReqBody>>(_handler: &T) {
        // no op
    }

    #[test]
    fn assert_fn_is_http_handler() {
        async fn get(_header: Method) -> () {
            ()
        }

        let http_handler = FnHandler::new(get);
        assert_is_fn_handler(&http_handler);
        assert_is_handler(&http_handler);
    }
}
