use async_trait::async_trait;
use crate::body::OptionReqBody;
use micro_http::protocol::ParseError;
use crate::RequestContext;

#[async_trait]
pub trait FromRequest {
    type Output<'r> : Send;
    async fn from_request<'r>(req: &'r RequestContext, body: OptionReqBody) -> Result<Self::Output<'r>, ParseError>;
}

#[async_trait]
impl<T> FromRequest for Option<T>
where
    T: FromRequest,
{
    type Output<'r> = Option<T::Output<'r>>;

    async fn from_request<'r>(req: &'r RequestContext, body: OptionReqBody) -> Result<Self::Output<'r>, ParseError> {
        match T::from_request(req, body.clone()).await {
            Ok(t) => Ok(Some(t)),
            Err(_) => Ok(None),
        }
    }
}

#[async_trait]
impl<T> FromRequest for Result<T, ParseError>
where
    T: FromRequest,
{
    type Output<'r> = Result<T::Output<'r>, ParseError>;

    async fn from_request<'r>(req: &'r RequestContext, body: OptionReqBody) -> Result<Self::Output<'r>, ParseError> {
        match T::from_request(req, body.clone()).await {
            Ok(t) => Ok(Ok(t)),
            e => Ok(e),
        }
    }
}

/// impl `FromRequest` for tuples
///
/// for example, it will impl Fn(A, B) like this:
///
/// ```ignore
/// # #![feature(async_fn_in_trait)]
/// # use micro_http::protocol::{HttpError, ParseError, RequestHeader};
/// # use micro_web::{FromRequest, OptionReqBody};
///
/// impl<'r, A, B> FromRequest<'r> for (A, B)
/// where
///     A: FromRequest<'r>,
///     B: FromRequest<'r>,
/// {
///     type Output = (A::Output, B::Output);
///
///     async fn from_request(req: &'r RequestHeader, body: OptionReqBody) -> Result<Self::Output, ParseError> {
///         let a = A::from_request(req, body.clone()).await?;
///         let b = B::from_request(req, body.clone()).await?;
///         Ok((a, b))
///     }
/// }
/// ```
macro_rules! impl_from_request_for_fn ({ $($param:ident)* } => {
    #[async_trait::async_trait]
    impl<$($param,)*> FromRequest for ($($param,)*)
    where
        $($param: FromRequest,)*
        $(for <'any> $param::Output<'any>: Send,)*
    {
        type Output<'r> = ($($param::Output<'r>,)*);

        #[allow(unused_variables)]
        async fn from_request<'r>(req: &'r RequestContext, body: OptionReqBody) -> Result<Self::Output<'r>, ParseError> {
            Ok(($($param::from_request(req, body.clone()).await?,)*))
        }
    }
});

#[async_trait]
impl FromRequest for () {
    type Output<'r> = ();

    async fn from_request(_req: &RequestContext, _body: OptionReqBody) -> Result<Self::Output<'static>, ParseError> {
        Ok(())
    }
}


impl_from_request_for_fn! {A}
impl_from_request_for_fn! {A B}
impl_from_request_for_fn! {A B C}
impl_from_request_for_fn! { A B C D }
impl_from_request_for_fn! { A B C D E }
impl_from_request_for_fn! { A B C D E F }
impl_from_request_for_fn! { A B C D E F G }
impl_from_request_for_fn! { A B C D E F G H }
impl_from_request_for_fn! { A B C D E F G H I }
impl_from_request_for_fn! { A B C D E F G H I J }
impl_from_request_for_fn! { A B C D E F G H I J K }
impl_from_request_for_fn! { A B C D E F G H I J K L }
