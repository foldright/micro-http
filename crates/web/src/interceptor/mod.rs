mod encoding;

use std::marker::PhantomData;

pub use encoding::encoder::EncodeWrapper;

pub struct Wrappers<W, Tail, H> {
    head: W,
    tail: Tail,
    _phantom: PhantomData<H>,
}

pub struct IdentityWrapper;
pub type IdentityWrappers<H> = Wrappers<IdentityWrapper, IdentityWrapper, H>;

impl<H> Wrappers<IdentityWrapper, IdentityWrapper, H> {
    pub fn new() -> Self {
        Self { head: IdentityWrapper, tail: IdentityWrapper, _phantom: PhantomData }
    }
}

impl<W, Tail, H> Wrappers<W, Tail, H>
where
    W: Wrapper<H>,
    Tail: Wrapper<W::Out>,
{
    pub fn add<W2>(self, wrapper: W2) -> Wrappers<Self, W2, H>
    where
        W2: Wrapper<Tail::Out>,
    {
        Wrappers { head: self, tail: wrapper, _phantom: PhantomData }
    }
}

pub trait Wrapper<H> {
    type Out;
    fn wrap(&self, handler: H) -> Self::Out;
}

impl<H> Wrapper<H> for IdentityWrapper {
    type Out = H;

    #[inline]
    fn wrap(&self, handler: H) -> Self::Out {
        handler
    }
}

impl<H, W, Tail> Wrapper<H> for Wrappers<W, Tail, H>
where
    W: Wrapper<H>,
    Tail: Wrapper<W::Out>,
{
    type Out = Tail::Out;

    fn wrap(&self, handler: H) -> Self::Out {
        let new_handler = self.head.wrap(handler);
        self.tail.wrap(new_handler)
    }
}

#[cfg(test)]
mod tests {
    use crate::handler::RequestHandler;
    use crate::interceptor::encoding::encoder::EncodeWrapper;
    use crate::interceptor::{IdentityWrapper, Wrappers};

    #[test]
    fn test() {
        let wrapper: Wrappers<IdentityWrapper, IdentityWrapper, Box<dyn RequestHandler>> = Wrappers::new();
        let wrapper = wrapper.add(EncodeWrapper);
    }
}
