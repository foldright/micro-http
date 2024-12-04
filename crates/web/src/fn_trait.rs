

//! Function trait implementation for handling async request handlers
//! 
//! This module provides the [`FnTrait`] trait which is used to abstract over different types
//! of async functions that can serve as request handlers. It supports functions with varying
//! numbers of parameters (from 0 to 12) and ensures they can be used in an async context.
//!
//! # Examples
//!
//! ```no_run
//! // Handler with no parameters
//! async fn handler0() -> &'static str {
//!     "Hello World!"
//! }
//!
//! // Handler with two parameters
//! async fn handler2(param1: &str, param2: i32) -> String {
//!     format!("Hello {}, number: {}", param1, param2)
//! }
//! ```
//!
//! The trait is automatically implemented for functions that:
//! - Are async functions
//! - Return a Future
//! - Are Send + Sync
//! - Have 0-12 parameters

use std::future::Future;
/// A trait for abstracting over async functions with varying numbers of parameters.
///
/// This trait allows the web framework to work with different types of handler functions
/// in a uniform way, regardless of their parameter count or types.
///
/// # Type Parameters
///
/// * `Args`: The tuple type containing all function parameters
///
/// # Associated Types
///
/// * `Output`: The type that the function returns when resolved
/// * `Fut`: The specific Future type that the function returns
///
/// # Examples
///
/// ```rust
/// use http::Method;///
///
/// use micro_web::FnTrait;
///
/// async fn my_handler(method: &Method) -> String {
///     format!("Handling {} request", method)
/// }
///
/// // The function automatically implements FnTrait
/// fn assert_handler<'r, F: FnTrait<(&'r Method,)>>(f: F) {}
/// assert_handler(my_handler);
/// ```
pub trait FnTrait<Args>: Send + Sync {
    type Output;
    type Fut: Future<Output = Self::Output> + Send;
    fn call(&self, args: Args) -> Self::Fut;
}

/// Implements `FnTrait` for functions with varying numbers of parameters.
///
/// This macro generates implementations for functions with 0 to 12 parameters,
/// allowing them to be used as request handlers in the web framework.
///
/// The generated implementations ensure that:
/// - The function is Send + Sync
/// - The returned Future is Send
/// - Parameters are properly passed through to the function
///
/// # Example Generated Implementation
///
/// ```ignore
/// // For a two-parameter function:
/// use micro_web::FnTrait;
///
/// impl<Func, Fut, A, B> FnTrait<(A, B)> for Func
/// where
///     Func: Fn(A, B) -> Fut + Send + Sync,
///     Fut: std::future::Future + Send,
/// {
///     type Output = Fut::Output;
///     type Fut = Fut;
///
///     fn call(&self, (a, b): (A, B)) -> Self::Fut {
///         (self)(a, b)
///     }
/// }
/// ```
macro_rules! impl_fn_trait_for_fn ({ $($param:ident)* } => {
    impl<Func, Fut, $($param,)*> FnTrait<($($param,)*)> for Func
    where
        Func: Fn($($param),*) -> Fut + Send + Sync ,
        Fut: std::future::Future + Send,
    {
        type Output = Fut::Output;
        type Fut = Fut;

        #[inline]
        #[allow(non_snake_case)]
        fn call(&self, ($($param,)*): ($($param,)*)) -> Self::Fut {
            (self)($($param,)*)
        }
    }
});

impl_fn_trait_for_fn! {}
impl_fn_trait_for_fn! { A }
impl_fn_trait_for_fn! { A B }
impl_fn_trait_for_fn! { A B C }
impl_fn_trait_for_fn! { A B C D }
impl_fn_trait_for_fn! { A B C D E }
impl_fn_trait_for_fn! { A B C D E F }
impl_fn_trait_for_fn! { A B C D E F G }
impl_fn_trait_for_fn! { A B C D E F G H }
impl_fn_trait_for_fn! { A B C D E F G H I }
impl_fn_trait_for_fn! { A B C D E F G H I J }
impl_fn_trait_for_fn! { A B C D E F G H I J K }
impl_fn_trait_for_fn! { A B C D E F G H I J K L }

#[cfg(test)]
mod tests {
    use crate::fn_trait::FnTrait;
    use http::{HeaderMap, Method};

    fn assert_is_fn_trait<Args, F: FnTrait<Args>>(_f: F) {
        //noop
    }
    async fn foo0() {}
    async fn foo1(_a: ()) {}
    async fn foo2(_a1: &Method, _a2: &HeaderMap) {}
    async fn foo3(_a1: &Method, _a2: &HeaderMap, _a3: ()) {}
    async fn foo4(_a1: &Method, _a2: &HeaderMap, _a3: (), _a4: ()) {}
    async fn foo5(_a1: (), _a2: &HeaderMap, _a3: (), _a4: (), _a5: ()) {}
    async fn foo6(_a1: (), _a2: &HeaderMap, _a3: (), _a4: (), _a5: (), _a6: ()) {}
    async fn foo7(_a1: &Method, _a2: (), _a3: (), _a4: (), _a5: (), _a6: (), _a7: ()) {}
    #[allow(clippy::too_many_arguments)]
    async fn foo8(_a1: &Method, _a2: &HeaderMap, _a3: (), _a4: (), _a5: (), _a6: (), _a7: (), _a8: ()) {}
    #[allow(clippy::too_many_arguments)]
    async fn foo9(_a1: &Method, _a2: (), _a3: (), _a4: (), _a5: (), _a6: (), _a7: (), _a8: (), _a9: ()) {}
    #[allow(clippy::too_many_arguments)]
    async fn foo10(
        _a1: &Method,
        _a2: &HeaderMap,
        _a3: (),
        _a4: (),
        _a5: (),
        _a6: (),
        _a7: (),
        _a8: (),
        _a9: (),
        _a10: (),
    ) {
    }
    #[allow(clippy::too_many_arguments)]
    async fn foo11(
        _a1: &Method,
        _a2: &HeaderMap,
        _a3: (),
        _a4: (),
        _a5: (),
        _a6: (),
        _a7: (),
        _a8: (),
        _a9: (),
        _a10: (),
        _a11: (),
    ) {
    }
    #[allow(clippy::too_many_arguments)]
    async fn foo12(
        _a1: &Method,
        _a2: &HeaderMap,
        _a3: (),
        _a4: (),
        _a5: (),
        _a6: (),
        _a7: (),
        _a8: (),
        _a9: (),
        _a10: (),
        _a11: (),
        _a12: (),
    ) {
    }

    #[test]
    fn test_fn_is_fn_trait() {
        assert_is_fn_trait(foo0);
        assert_is_fn_trait(foo1);
        assert_is_fn_trait(foo2);
        assert_is_fn_trait(foo3);
        assert_is_fn_trait(foo4);
        assert_is_fn_trait(foo5);
        assert_is_fn_trait(foo6);
        assert_is_fn_trait(foo7);
        assert_is_fn_trait(foo8);
        assert_is_fn_trait(foo9);
        assert_is_fn_trait(foo10);
        assert_is_fn_trait(foo11);
        assert_is_fn_trait(foo12);
    }
}
