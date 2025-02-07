use crate::decorator::{Decorator, IdentityDecorator};

pub struct DecoratorComposer<D1, D2> {
    decorator_1: D1,
    decorator_2: D2,
}

impl<D1, D2> DecoratorComposer<D1, D2> {
    pub fn new(decorator_1: D1, decorator_2: D2) -> Self {
        Self { decorator_1, decorator_2 }
    }
}

impl Default for DecoratorComposer<IdentityDecorator, IdentityDecorator> {
    fn default() -> Self {
        Self::new(IdentityDecorator, IdentityDecorator)
    }
}

impl<In, D1, D2> Decorator<In> for DecoratorComposer<D1, D2>
where
    D1: Decorator<In>,
    D2: Decorator<D1::Out>,
{
    type Out = D2::Out;

    fn decorate(&self, raw: In) -> Self::Out {
        let output_1 = self.decorator_1.decorate(raw);
        self.decorator_2.decorate(output_1)
    }
}
