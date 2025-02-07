mod decorator_composer;
mod decorator_fn;
mod identity;

pub use decorator_composer::DecoratorComposer;
pub use identity::IdentityDecorator;

pub trait Decorator<In> {
    type Out;

    fn decorate(&self, raw: In) -> Self::Out;
}

pub trait DecoratorExt<In>: Decorator<In> {
    fn and_then<D>(self, decorator: D) -> DecoratorComposer<Self, D>
    where
        Self: Sized,
    {
        DecoratorComposer::new(self, decorator)
    }

    fn compose<D>(self, decorator: D) -> DecoratorComposer<D, Self>
    where
        Self: Sized,
    {
        DecoratorComposer::new(decorator, self)
    }
}

impl<T: Decorator<In> + ?Sized, In> DecoratorExt<In> for T {}
