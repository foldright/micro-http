use http::{HeaderMap, Method};
use micro_http::protocol::body::ReqBody;
use micro_http::protocol::{HttpError, RequestHeader};

pub trait FromRequest: Sized {
    async fn from_request(req: &RequestHeader, body: &mut ReqBody) -> Result<Self, HttpError>;
}

impl FromRequest for Method {
    async fn from_request(req: &RequestHeader, _body: &mut ReqBody) -> Result<Self, HttpError> {
        Ok(req.method().clone())
    }
}

impl FromRequest for HeaderMap {
    async fn from_request(req: &RequestHeader, _body: &mut ReqBody) -> Result<Self, HttpError> {
        Ok(req.headers().clone())
    }
}

macro_rules! impl_from_request_for_fn ({ $($param:ident)* } => {
    impl<$($param,)*> FromRequest for ($($param,)*)
    where
        $($param: FromRequest,)*
    {
        async fn from_request(req: &RequestHeader, body: &mut ReqBody) -> Result<Self, HttpError> {
            Ok(($($param::from_request(req, body).await.unwrap(),)*))
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

impl<T> FromRequest for Option<T>
where
    T: FromRequest,
{
    async fn from_request(req: &RequestHeader, body: &mut ReqBody) -> Result<Self, HttpError> {
        match T::from_request(req, body).await {
            Ok(t) => Ok(Some(t)),
            Err(_) => Ok(None),
        }
    }
}

impl<T> FromRequest for Result<T, HttpError>
where
    T: FromRequest,
{
    async fn from_request(req: &RequestHeader, body: &mut ReqBody) -> Result<Self, HttpError> {
        Ok(T::from_request(req, body).await)
    }
}
