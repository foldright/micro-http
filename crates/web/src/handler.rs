use crate::body::ResponseBody;
use crate::fn_trait::FnTrait;
use crate::request::PathParams;
use crate::responder::Responder;
use crate::{FromRequest, OptionReqBody, RequestContext};
use async_trait::async_trait;
use http::{Request, Response};
use matchit::Params;
use micro_http::handler::Handler;
use micro_http::protocol::body::ReqBody;
use micro_http::protocol::RequestHeader;
use std::error::Error;
use std::marker::PhantomData;

#[async_trait]
pub trait RequestHandler: Send + Sync {
    async fn invoke<'server, 'req>(
        &self,
        req: RequestContext<'server, 'req>,
        req_body: OptionReqBody,
    ) -> Result<Response<ResponseBody>, Box<dyn Error + Send + Sync>>;
}

/// a `FnTrait` holder which represents any async Fn
pub struct FnHandler<F, Args> {
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

// impl<F, Args> From<F> for FnHandler<F, Args>
// where
//     Args: FromRequest,
//     F: for<'r> FnTrait<Args::Output<'r>>,
//     for<'r> <F as FnTrait<Args::Output<'r>>>::Output: Responder,
// {
//     fn from(f: F) -> Self {
//         Self { f, _phantom: PhantomData }
//     }
// }
//
// impl<F, Args> From<FnHandler<F, Args>> for Box<dyn RequestHandler>
// where
//     Args: FromRequest + 'static,
//     F: for<'r> FnTrait<Args::Output<'r>> + 'static,
//     for<'r> <F as FnTrait<Args::Output<'r>>>::Output: Responder,
// {
//     fn from(fn_handler: FnHandler<F, Args>) -> Self {
//         Box::new(fn_handler)
//     }
// }

#[async_trait]
impl<F, Args> RequestHandler for FnHandler<F, Args>
where
    Args: FromRequest,
    F: for<'r> FnTrait<Args::Output<'r>>,
    for<'r> <F as FnTrait<Args::Output<'r>>>::Output: Responder,
{
    async fn invoke<'server, 'req>(
        &self,
        req: RequestContext<'server, 'req>,
        req_body: OptionReqBody,
    ) -> Result<Response<ResponseBody>, Box<dyn Error + Send + Sync>> {
        let args = Args::from_request(&req, req_body.clone()).await?;
        let responder = self.f.call(args).await;
        Ok(responder.response_to(&req))
    }
}

#[cfg(test)]
mod test {
    use crate::fn_trait::FnTrait;
    use crate::handler::{FnHandler, RequestHandler};
    use crate::request::PathParams;
    use http::Method;
    use micro_http::handler::Handler;
    use micro_http::protocol::body::ReqBody;

    fn assert_is_fn_handler<H: FnTrait<Args>, Args>(_handler: &FnHandler<H, Args>) {
        // no op
    }

    fn assert_is_handler<T: RequestHandler>(_handler: &T) {
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
