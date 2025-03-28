use crate::handler::RequestHandler;
use crate::router::handler_decorator::{IdentityRequestHandlerDecorator, RequestHandlerDecorator, RequestHandlerDecoratorComposer};

pub trait RequestHandlerDecoratorFactory: Sized {
    type Output<In>: RequestHandlerDecorator<In> + 'static
    where
        In: RequestHandler;

    fn create_decorator<In>(&self) -> Self::Output<In>
    where
        In: RequestHandler;
}

pub trait RequestHandlerDecoratorFactoryExt: RequestHandlerDecoratorFactory {
    fn and_then<F: RequestHandlerDecoratorFactory>(self, factory: F) -> RequestHandlerDecoratorFactoryComposer<Self, F> {
        RequestHandlerDecoratorFactoryComposer { factory_1: self, factory_2: factory }
    }

    fn compose<F: RequestHandlerDecoratorFactory>(self, factory: F) -> RequestHandlerDecoratorFactoryComposer<F, Self> {
        RequestHandlerDecoratorFactoryComposer { factory_1: factory, factory_2: self }
    }
}

impl <T: RequestHandlerDecoratorFactory> RequestHandlerDecoratorFactoryExt for T {}

#[derive(Debug, Default, Copy, Clone)]
pub struct IdentityRequestHandlerDecoratorFactory;

impl RequestHandlerDecoratorFactory for IdentityRequestHandlerDecoratorFactory {
    type Output<In>
        = IdentityRequestHandlerDecorator
    where
        In: RequestHandler;

    fn create_decorator<In>(&self) -> Self::Output<In>
    where
        In: RequestHandler,
    {
        IdentityRequestHandlerDecorator
    }
}

pub struct RequestHandlerDecoratorFactoryComposer<F1, F2> {
    factory_1: F1,
    factory_2: F2,
}

impl<F1, F2> RequestHandlerDecoratorFactory for RequestHandlerDecoratorFactoryComposer<F1, F2>
where
    F1: RequestHandlerDecoratorFactory,
    F2: RequestHandlerDecoratorFactory,
{
    // F1::Output<In> means: first factory's output, which is the first Decorator
    // F2::Output<<F1::Output<In> as RequestHandlerDecorator<In>>::Out> means:
    // 1.  `<F1::Output<In> as RequestHandlerDecorator<In>` means: the first Decorator
    // 2.  `<F1::Output<In> as RequestHandlerDecorator<In>>::Out` means: the first Decorator's result
    // 3.  `Output<<F1::Output<In> as RequestHandlerDecorator<In>>::Out>` means: the second Decorator's param is the first Decorator's result
    type Output<In>
        = RequestHandlerDecoratorComposer<F1::Output<In>, F2::Output<<F1::Output<In> as RequestHandlerDecorator<In>>::Output>>
    where
        In: RequestHandler;

    fn create_decorator<In>(&self) -> Self::Output<In>
    where
        In: RequestHandler,
    {
        let decorator_1 = self.factory_1.create_decorator();
        let decorator_2 = self.factory_2.create_decorator();

        RequestHandlerDecoratorComposer::new(decorator_1, decorator_2)
    }
}
