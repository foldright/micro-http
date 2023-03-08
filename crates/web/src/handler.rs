use crate::responder::Responder;
use crate::FromRequest;
use http::{Request, Response};
use http_body::Body;
use micro_http::handler::Handler;
use micro_http::protocol::body::ReqBody;
use micro_http::protocol::RequestHeader;
use std::error::Error;
use std::future::Future;
use std::marker::PhantomData;

pub trait HttpHandler<Args> {
    type Output;
    async fn call(&self, args: Args) -> Self::Output;
}

struct HttpHandlerHolder<Args, H: HttpHandler<Args>> {
    handler: H,
    _phantom: PhantomData<fn(Args)>,
}

impl<Args, H, RespBody> HttpHandlerHolder<Args, H>
where
    H: HttpHandler<Args>,
    H::Output: Responder<Body = RespBody>,
    RespBody: Body,
    Args: FromRequest,
{
    pub fn new(handler: H) -> Self {
        Self { handler, _phantom: PhantomData }
    }
}

impl<Args, H, RespBody> Handler<ReqBody> for HttpHandlerHolder<Args, H>
where
    H: HttpHandler<Args>,
    H::Output: Responder<Body = RespBody>,
    RespBody: Body,
    Args: FromRequest,
{
    type RespBody = RespBody;
    type Error = Box<dyn Error + Send + Sync>;

    async fn call(&self, req: Request<ReqBody>) -> Result<Response<Self::RespBody>, Self::Error> {
        let (parts, mut body) = req.into_parts();
        let header = RequestHeader::from(parts);
        let args = Args::from_request(&header, &mut body).await?;
        let responder = self.handler.call(args).await;
        Ok(responder.response_to(&header))
    }
}

macro_rules! impl_http_handler_for_fn ({ $($param:ident)* } => {
    impl<Func, Fut, $($param,)*> HttpHandler<($($param,)*)> for Func
    where
        Func: Fn($($param),*) -> Fut,
        Fut: Future,
    {
        type Output = Fut::Output;

        #[inline]
        #[allow(non_snake_case)]
        async fn call(&self, ($($param,)*): ($($param,)*)) -> Self::Output {
            (self)($($param,)*).await
        }
    }
});

impl_http_handler_for_fn! {}
impl_http_handler_for_fn! { A }
impl_http_handler_for_fn! { A B }
impl_http_handler_for_fn! { A B C }
impl_http_handler_for_fn! { A B C D }
impl_http_handler_for_fn! { A B C D E }
impl_http_handler_for_fn! { A B C D E F }
impl_http_handler_for_fn! { A B C D E F G }
impl_http_handler_for_fn! { A B C D E F G H }
impl_http_handler_for_fn! { A B C D E F G H I }
impl_http_handler_for_fn! { A B C D E F G H I J }
impl_http_handler_for_fn! { A B C D E F G H I J K }
impl_http_handler_for_fn! { A B C D E F G H I J K L }

#[cfg(test)]
mod test {
    use crate::handler::{HttpHandler, HttpHandlerHolder};
    use crate::responder::Responder;
    use bytes::Bytes;
    use http::{Method, Response};
    use http_body_util::Empty;
    use micro_http::handler::Handler;
    use micro_http::protocol::body::ReqBody;
    use micro_http::protocol::RequestHeader;
    use std::error::Error;
    use std::marker::PhantomData;

    fn assert_is_http_handler<Args, T: HttpHandler<Args>>(handler: T) {
        // no op
    }

    fn assert_is_http_handler_holder<Args, H: HttpHandler<Args>>(handler: HttpHandlerHolder<Args, H>) {
        // no op
    }

    fn assert_is_http_handler_holder_is_handler<T: Handler<ReqBody>>(handler: T) {
        // no op
    }

    #[test]
    fn assert_fn_is_http_handler() {
        async fn get(header: Method) -> () {
            ()
        }

        assert_is_http_handler(get);

        let http_handler = HttpHandlerHolder { handler: get, _phantom: PhantomData };
        assert_is_http_handler_holder(http_handler);

        let http_handler_holder = HttpHandlerHolder::new(get);

        assert_is_http_handler_holder_is_handler(http_handler_holder);
    }
}
