use crate::handler::RequestHandler;

pub trait HandlerDecorator<In: RequestHandler> {
    type Output: RequestHandler;

    fn decorate(&self, handler: In) -> Self::Output;
}


#[allow(unused)]
pub trait HandlerDecoratorExt<In: RequestHandler>: HandlerDecorator<In> {
    fn and_then<D>(self, decorator: D) -> HandlerDecoratorComposer<Self, D>
    where
        Self: Sized,
    {
        HandlerDecoratorComposer::new(self, decorator)
    }

    fn compose<D>(self, decorator: D) -> HandlerDecoratorComposer<D, Self>
    where
        Self: Sized,
    {
        HandlerDecoratorComposer::new(decorator, self)
    }
}

impl<T: HandlerDecorator<In> + ?Sized, In: RequestHandler> HandlerDecoratorExt<In> for T {}

#[derive(Default, Copy, Clone, Debug)]
pub struct IdentityHandlerDecorator;

impl<In: RequestHandler> HandlerDecorator<In> for IdentityHandlerDecorator {
    type Output = In;

    fn decorate(&self, handler: In) -> Self::Output {
        handler
    }
}

pub struct HandlerDecoratorComposer<D1, D2> {
    decorator_1: D1,
    decorator_2: D2,
}

impl<D1, D2> HandlerDecoratorComposer<D1, D2> {
    pub fn new(decorator_1: D1, decorator_2: D2) -> Self {
        HandlerDecoratorComposer { decorator_1, decorator_2 }
    }
}

impl<In, D1, D2> HandlerDecorator<In> for HandlerDecoratorComposer<D1, D2>
where
    In: RequestHandler,
    D1: HandlerDecorator<In>,
    D2: HandlerDecorator<D1::Output>,
{
    type Output = D2::Output;
    fn decorate(&self, handler: In) -> Self::Output {
        let output_1 = self.decorator_1.decorate(handler);
        self.decorator_2.decorate(output_1)
    }
}
