use crate::decorator::Decorator;

#[derive(Copy, Clone)]
pub struct DecoratorFn<F> {
    f: F,
}

#[allow(unused)]
pub fn decorator_fn<In, Out, F>(f: F) -> DecoratorFn<F>
where
    F: Fn(In) -> Out,
{
    DecoratorFn { f }
}

impl<In, Out, F> Decorator<In> for DecoratorFn<F>
where
    F: Fn(In) -> Out,
{
    type Out = Out;
    fn decorate(&self, raw: In) -> Self::Out {
        (self.f)(raw)
    }
}
