mod date;
mod encoding;

use std::marker::PhantomData;

pub use date::DateWrapper;
pub use encoding::encoder::EncodeWrapper;

/// A wrapper that can wrap a handler to another
pub trait Wrapper<H> {
    /// the wrapper's output
    type Out;

    /// wrap the handler to another
    fn wrap(&self, handler: H) -> Self::Out;
}

/// A list of [`Wrapper`], which will wrap a handler to another
pub struct Wrappers<Head, Tail, H> {
    head: Head,
    tail: Tail,
    _phantom: PhantomData<H>,
}

/// An identity wrappers, which does not do any wrapping
pub type IdentityWrappers<H> = Wrappers<IdentityWrapper, IdentityWrapper, H>;

impl<H> IdentityWrappers<H> {
    fn new() -> Self {
        Self { head: IdentityWrapper, tail: IdentityWrapper, _phantom: PhantomData }
    }
}

impl<H> Default for IdentityWrappers<H> {
    fn default() -> Self {
        Self::new()
    }
}

/// An identity wrapper, which does not do any wrapping
pub struct IdentityWrapper;
impl<H> Wrapper<H> for IdentityWrapper {
    type Out = H;

    #[inline]
    fn wrap(&self, handler: H) -> Self::Out {
        handler
    }
}

impl<Head, Tail, H> Wrappers<Head, Tail, H>
where
    Head: Wrapper<H>,
    Tail: Wrapper<Head::Out>,
{
    /// add a [`Wrapper`] to the end of the [`Wrappers`], the argument [`Wrapper`] will wrap at last
    pub fn and_then<NewW>(self, wrapper: NewW) -> Wrappers<Self, NewW, H>
    where
        NewW: Wrapper<Tail::Out>,
    {
        Wrappers { head: self, tail: wrapper, _phantom: PhantomData }
    }
}

impl<Head, Tail, H> Wrapper<H> for Wrappers<Head, Tail, H>
where
    Head: Wrapper<H>,
    Tail: Wrapper<Head::Out>,
{
    type Out = Tail::Out;

    fn wrap(&self, handler: H) -> Self::Out {
        let new_handler = self.head.wrap(handler);
        self.tail.wrap(new_handler)
    }
}

#[cfg(test)]
mod tests {
    use crate::wrapper::{IdentityWrapper, Wrapper, Wrappers};

    trait Service {
        fn call(&self, input: String) -> String;
    }

    struct Service0;
    struct Service1<S: Service>(S);
    struct Service2<S: Service>(S);

    impl Service for Service0 {
        fn call(&self, input: String) -> String {
            format!("s0 {input}")
        }
    }

    impl<S: Service> Service for Service1<S> {
        fn call(&self, input: String) -> String {
            let result = self.0.call(input);
            format!("s1 {result}")
        }
    }

    impl<S: Service> Service for Service2<S> {
        fn call(&self, input: String) -> String {
            let result = self.0.call(input);
            format!("s2 {result}")
        }
    }

    struct Service1Wrapper;
    struct Service2Wrapper;

    impl<S: Service> Wrapper<S> for Service1Wrapper {
        type Out = Service1<S>;

        fn wrap(&self, handler: S) -> Self::Out {
            Service1(handler)
        }
    }

    impl<S: Service> Wrapper<S> for Service2Wrapper {
        type Out = Service2<S>;

        fn wrap(&self, handler: S) -> Self::Out {
            Service2(handler)
        }
    }

    #[test]
    fn test_and_then() {
        let wrappers: Wrappers<IdentityWrapper, IdentityWrapper, Service0> = Wrappers::default();

        let wrappers = wrappers.and_then(Service1Wrapper).and_then(Service2Wrapper);

        let service = wrappers.wrap(Service0);

        let result = service.call("Hello".into());
        assert_eq!(result, format!("s2 s1 s0 Hello"));
    }
}
