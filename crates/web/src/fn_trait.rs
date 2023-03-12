/// Represents a function
pub trait FnTrait<Args> {
    type Output;
    async fn call(&self, args: Args) -> Self::Output;
}

/// impl `Fn` for `FnTrait`, From 0 parameters to 12 parameters
///
/// for example, it will impl Fn(A, B) like this:
///```no_run
/// impl<Func, Fut, A, B> FnTrait<(A, B)> for Func
///    where
///        Func: Fn(A, B) -> Fut,
///        Fut: std::future::Future,
/// {
///    type Output = Fut::Output;
///
///    #[inline]
///    #[allow(non_snake_case)]
///    async fn call(&self, (A, B): (A, B)) -> Self::Output {
///        (self)(A, B)
///    }
/// }
///```
macro_rules! impl_fn_trait_for_fn ({ $($param:ident)* } => {
    impl<Func, Fut, $($param,)*> FnTrait<($($param,)*)> for Func
    where
        Func: Fn($($param),*) -> Fut,
        Fut: std::future::Future,
    {
        type Output = Fut::Output;

        #[inline]
        #[allow(non_snake_case)]
        async fn call(&self, ($($param,)*): ($($param,)*)) -> Self::Output {
            (self)($($param,)*).await
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
    async fn foo8(_a1: &Method, _a2: &HeaderMap, _a3: (), _a4: (), _a5: (), _a6: (), _a7: (), _a8: ()) {}
    async fn foo9(_a1: &Method, _a2: (), _a3: (), _a4: (), _a5: (), _a6: (), _a7: (), _a8: (), _a9: ()) {}
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
