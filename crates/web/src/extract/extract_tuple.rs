use crate::body::OptionReqBody;
use crate::responder::Responder;
use crate::{RequestContext, ResponseBody};
use http::Response;
use crate::extract::from_request::FromRequest2;


macro_rules! impl_from_request_2_for_fn {
    ($either:ident, $($param:ident)*) => {
        impl<$($param,)*> FromRequest2 for ($($param,)*)
        where
            $($param: FromRequest2,)*
            $(for <'any> $param::Output<'any>: Send,)*
        {
            type Output<'r> = ($($param::Output<'r>,)*);
            type Error = $either<$($param::Error,)*>;

            #[allow(non_snake_case)]
            async fn from_request<'r>(req: &'r RequestContext<'_, '_>, body: OptionReqBody) -> Result<Self::Output<'r>, Self::Error> {
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

impl_from_request_2_for_fn! { EitherA2, A }
impl_from_request_2_for_fn! { EitherAB2, A B}
impl_from_request_2_for_fn! { EitherABC2, A B C}
impl_from_request_2_for_fn! { EitherABCD2, A B C D }
impl_from_request_2_for_fn! { EitherABCDE2, A B C D E }
impl_from_request_2_for_fn! { EitherABCDEF2, A B C D E F }
impl_from_request_2_for_fn! { EitherABCDEFG2, A B C D E F G }
impl_from_request_2_for_fn! { EitherABCDEFGH2, A B C D E F G H }
impl_from_request_2_for_fn! { EitherABCDEFGHI2, A B C D E F G H I }
impl_from_request_2_for_fn! { EitherABCDEFGHIJ2, A B C D E F G H I J }
impl_from_request_2_for_fn! { EitherABCDEFGHIJK2, A B C D E F G H I J K }
impl_from_request_2_for_fn! { EitherABCDEFGHIJKL2, A B C D E F G H I J K L }

