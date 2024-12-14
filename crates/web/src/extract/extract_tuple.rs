//! Tuple data extraction implementations
//! 
//! This module provides implementations for extracting typed data from HTTP requests into tuples.
//! It supports extracting multiple parameters from a single request and combining them into a tuple.

use crate::body::OptionReqBody;
use crate::extract::FromRequest;
use crate::responder::Responder;
use crate::{RequestContext, ResponseBody};
use http::Response;

/// impl `FromRequest` for tuples
///
/// for example, it will impl Fn(A, B) like this:
///
/// ```ignore
///#[async_trait]
/// impl<A, B> FromRequest for (A, B)
/// where
///     A: FromRequest,
///     B: FromRequest,
/// {
///     type Output<'any> = (A::Output<'any>, B::Output<'any>);
///     type Error = EitherAB<A::Error, B::Error>;
///
///     #[allow(non_snake_case)]
///     async fn from_request<'r>(req: &'r RequestContext, body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error> {
///         let A = A::from_request(req, body.clone()).await.map_err(EitherAB::A)?;
///         let B = B::from_request(req, body.clone()).await.map_err(EitherAB::B)?;
///         Ok((A, B))
///     }
/// }
///
/// pub enum EitherAB<A, B> {
///     A(A),
///     B(B),
/// }
///
/// impl<A, B> EitherAB<A, B> {
///     #[allow(non_snake_case)]
///     fn A(A: A) -> Self {
///         EitherAB::A(A)
///     }
///
///     #[allow(non_snake_case)]
///     fn B(B: B) -> Self {
///         EitherAB::B(B)
///     }
/// }
///
/// impl<A, B> Responder for EitherAB<A, B>
/// where
///     A: Responder,
///     B: Responder,
/// {
///     #[allow(non_snake_case)]
///     fn response_to(self, req: &RequestContext) -> Response<ResponseBody> {
///         match self {
///             EitherAB::A(A) => A.response_to(req),
///             EitherAB::B(B) => B.response_to(req),
///         }
///     }
/// }
/// ```
macro_rules! impl_from_request_for_fn {
    ($either:ident, $($param:ident)*) => {
        #[async_trait::async_trait]
        impl<$($param,)*> FromRequest for ($($param,)*)
        where
            $($param: FromRequest,)*
            $(for <'any> $param::Output<'any>: Send,)*
        {
            type Output<'r> = ($($param::Output<'r>,)*);
            type Error = $either<$($param::Error,)*>;

            #[allow(non_snake_case)]
            async fn from_request<'r>(req: &'r RequestContext, body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error> {
                Ok(($($param::from_request(req, body.clone()).await.map_err($either::$param)?,)*))
            }
        }

        pub enum $either<$($param,)*> {
            $(
            $param($param),
            )*
        }

        impl<$($param,)*> $either<$($param,)*> {
            $(
            #[allow(dead_code)]
            #[allow(non_snake_case)]
            fn $param($param: $param) -> Self {
                $either::$param($param)
            }
            )*
        }

        impl<$($param,)*> Responder for $either<$($param,)*>
            where
                $(
                $param: Responder,
                )*
        {
            #[allow(non_snake_case)]
            fn response_to(self, req: &RequestContext) -> Response<ResponseBody> {
                match self {
                    $(
                        $either::$param($param) => $param.response_to(req),
                    )*
                }
            }
        }
    }
}

impl_from_request_for_fn! { EitherA, A }
impl_from_request_for_fn! { EitherAB, A B}
impl_from_request_for_fn! { EitherABC, A B C}
impl_from_request_for_fn! { EitherABCD, A B C D }
impl_from_request_for_fn! { EitherABCDE, A B C D E }
impl_from_request_for_fn! { EitherABCDEF, A B C D E F }
impl_from_request_for_fn! { EitherABCDEFG, A B C D E F G }
impl_from_request_for_fn! { EitherABCDEFGH, A B C D E F G H }
impl_from_request_for_fn! { EitherABCDEFGHI, A B C D E F G H I }
impl_from_request_for_fn! { EitherABCDEFGHIJ, A B C D E F G H I J }
impl_from_request_for_fn! { EitherABCDEFGHIJK, A B C D E F G H I J K }
impl_from_request_for_fn! { EitherABCDEFGHIJKL, A B C D E F G H I J K L }
