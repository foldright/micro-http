use http::{HeaderMap, Method};
use micro_http::protocol::body::ReqBody;
use micro_http::protocol::{HttpError, RequestHeader};

pub trait FromRequest<'r> {
    type Output;
    async fn from_request(req: &'r RequestHeader) -> Result<Self::Output, HttpError>;
}

/// impl `FromRequest` for tuples
///
/// for example, it will impl Fn(A, B) like this:
///
/// ```no_run
/// # use micro_http::protocol::{HttpError, RequestHeader};
/// # use micro_web::FromRequest;
///
/// impl<'r, A, B> FromRequest<'r> for (A, B)
/// where
///     A: FromRequest<'r>,
///     B: FromRequest<'r>,
/// {
///     type Output = (A::Output, B::Output);
///
///     async fn from_request(req: &'r RequestHeader) -> Result<Self::Output, HttpError> {
///         let a = A::from_request(req).await?;
///         let b = B::from_request(req).await?;
///         Ok((a, b))
///     }
/// }
/// ```
macro_rules! impl_from_request_for_fn ({ $($param:ident)* } => {
    impl<'r, $($param,)*> FromRequest<'r> for ($($param,)*)
    where
        $($param: FromRequest<'r>,)*
    {
        type Output = ($($param::Output,)*);
        async fn from_request(req: &'r RequestHeader) -> Result<Self::Output, HttpError> {

            Ok(($($param::from_request(req).await?,)*))
        }
    }
});

impl_from_request_for_fn! {}
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

impl<'r> FromRequest<'r> for Method {
    type Output = Method;

    async fn from_request(req: &'r RequestHeader) -> Result<Self::Output, HttpError> {
        Ok(req.method().clone())
    }
}
impl<'r> FromRequest<'r> for &RequestHeader {
    type Output = &'r RequestHeader;

    async fn from_request(req: &'r RequestHeader) -> Result<Self::Output, HttpError> {
        Ok(req)
    }
}

impl<'r> FromRequest<'r> for &HeaderMap {
    type Output = &'r HeaderMap;

    async fn from_request(req: &'r RequestHeader) -> Result<Self::Output, HttpError> {
        Ok(req.headers())
    }
}

// impl<T> FromRequest for Option<T>
// where
//     T: FromRequest,
// {
//     async fn from_request(req: &RequestHeader, body: &mut ReqBody) -> Result<Self, HttpError> {
//         match T::from_request(req, body).await {
//             Ok(t) => Ok(Some(t)),
//             Err(_) => Ok(None),
//         }
//     }
// }
//
// impl<T> FromRequest for Result<T, HttpError>
// where
//     T: FromRequest,
// {
//     async fn from_request(req: &RequestHeader, body: &mut ReqBody) -> Result<Self, HttpError> {
//         Ok(T::from_request(req, body).await)
//     }
// }
