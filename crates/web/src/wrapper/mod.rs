//! Module for request/response transformation wrappers.
//!
//! This module provides a composable wrapper system that allows transformation of request handlers.
//! Wrappers can be chained together to build a processing pipeline that modifies requests or responses.
//! Common use cases include adding headers, encoding responses, or performing cross-cutting concerns
//! like logging or metrics.
//!
//! # Key Components
//! - [`Wrapper`]: Core trait for implementing wrappers
//! - [`Wrappers`]: A composable list of wrappers that can be chained together
//! - [`IdentityWrapper`]: A no-op wrapper that passes through the handler unchanged
mod date;
mod encoding;

use std::marker::PhantomData;

pub use date::DateWrapper;
pub use encoding::encoder::EncodeWrapper;

/// A trait for transforming request handlers.
///
/// Implementors of this trait can wrap a handler to add additional processing
/// before or after the handler's execution. Multiple wrappers can be composed
/// together using the [`Wrappers`] type.
pub trait Wrapper<H> {
    /// The type of the wrapped handler
    type Out;

    /// Wraps the given handler with additional processing
    fn wrap(&self, handler: H) -> Self::Out;
}

/// A composable list of wrappers that can transform a handler.
///
/// `Wrappers` allows multiple [`Wrapper`]s to be chained together using [`and_then`](Wrappers::and_then).
/// The wrappers are applied in the order they are added, with each wrapper potentially transforming
/// both the request and response.
pub struct Wrappers<Head, Tail, H> {
    head: Head,
    tail: Tail,
    _phantom: PhantomData<H>,
}

/// An identity wrapper chain that performs no transformations.
///
/// This is the default starting point for building a wrapper chain.
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

/// A wrapper that performs no transformation on the handler.
///
/// This is used as the default wrapper when creating a new [`Wrappers`] chain.
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
    /// Adds a new wrapper to the end of this wrapper chain.
    ///
    /// The new wrapper will be applied after all existing wrappers in the chain.
    /// This allows building up a processing pipeline step by step.
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
