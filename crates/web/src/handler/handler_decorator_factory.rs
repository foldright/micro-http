use crate::handler::RequestHandler;
use crate::handler::handler_decorator::{HandlerDecorator, HandlerDecoratorComposer, IdentityHandlerDecorator};

pub trait HandlerDecoratorFactory: Sized {
    type Output<In>: HandlerDecorator<In> + 'static
    where
        In: RequestHandler;

    fn create_decorator<In>(&self) -> Self::Output<In>
    where
        In: RequestHandler;
}

pub trait HandlerDecoratorFactoryExt: HandlerDecoratorFactory {
    fn and_then<F: HandlerDecoratorFactory>(self, factory: F) -> HandlerDecoratorFactoryComposer<Self, F> {
        HandlerDecoratorFactoryComposer { factory_1: self, factory_2: factory }
    }

    #[allow(unused)]
    fn compose<F: HandlerDecoratorFactory>(self, factory: F) -> HandlerDecoratorFactoryComposer<F, Self> {
        HandlerDecoratorFactoryComposer { factory_1: factory, factory_2: self }
    }
}

impl<T: HandlerDecoratorFactory> HandlerDecoratorFactoryExt for T {}

#[derive(Debug, Default, Copy, Clone)]
pub struct IdentityHandlerDecoratorFactory;

impl HandlerDecoratorFactory for IdentityHandlerDecoratorFactory {
    type Output<In>
        = IdentityHandlerDecorator
    where
        In: RequestHandler;

    fn create_decorator<In>(&self) -> Self::Output<In>
    where
        In: RequestHandler,
    {
        IdentityHandlerDecorator
    }
}

#[derive(Debug)]
pub struct HandlerDecoratorFactoryComposer<F1, F2> {
    factory_1: F1,
    factory_2: F2,
}

impl<F1, F2> HandlerDecoratorFactory for HandlerDecoratorFactoryComposer<F1, F2>
where
    F1: HandlerDecoratorFactory,
    F2: HandlerDecoratorFactory,
{
    // F1::Output<In> means: first factory's output, which is the first Decorator
    // F2::Output<<F1::Output<In> as HandlerDecorator<In>>::Out> means:
    // 1.  `<F1::Output<In> as HandlerDecorator<In>` means: the first Decorator
    // 2.  `<F1::Output<In> as HandlerDecorator<In>>::Out` means: the first Decorator's result
    // 3.  `Output<<F1::Output<In> as HandlerDecorator<In>>::Out>` means: the second Decorator's param is the first Decorator's result
    type Output<In>
        = HandlerDecoratorComposer<F1::Output<In>, F2::Output<<F1::Output<In> as HandlerDecorator<In>>::Output>>
    where
        In: RequestHandler;

    fn create_decorator<In>(&self) -> Self::Output<In>
    where
        In: RequestHandler,
    {
        let decorator_1 = self.factory_1.create_decorator();
        let decorator_2 = self.factory_2.create_decorator();

        HandlerDecoratorComposer::new(decorator_1, decorator_2)
    }
}
