use crate::handler::RequestHandler;

pub trait RequestHandlerDecorator<In: RequestHandler> {
    type Output: RequestHandler;

    fn decorate(&self, handler: In) -> Self::Output;
}

pub trait RequestHandlerDecoratorExt<In: RequestHandler>: RequestHandlerDecorator<In> {
    fn and_then<D>(self, decorator: D) -> RequestHandlerDecoratorComposer<Self, D>
    where
        Self: Sized,
    {
        RequestHandlerDecoratorComposer::new(self, decorator)
    }

    fn compose<D>(self, decorator: D) -> RequestHandlerDecoratorComposer<D, Self>
    where
        Self: Sized,
    {
        RequestHandlerDecoratorComposer::new(decorator, self)
    }
}

impl<T: RequestHandlerDecorator<In> + ?Sized, In: RequestHandler> RequestHandlerDecoratorExt<In> for T {}

#[derive(Default, Copy, Clone, Debug)]
pub struct IdentityRequestHandlerDecorator;

impl<In: RequestHandler> RequestHandlerDecorator<In> for IdentityRequestHandlerDecorator {
    type Output = In;

    fn decorate(&self, handler: In) -> Self::Output {
        handler
    }
}

pub struct RequestHandlerDecoratorComposer<D1, D2> {
    decorator_1: D1,
    decorator_2: D2,
}

impl<D1, D2> RequestHandlerDecoratorComposer<D1, D2> {
    pub fn new(decorator_1: D1, decorator_2: D2) -> Self {
        RequestHandlerDecoratorComposer { decorator_1, decorator_2 }
    }
}

impl<In, D1, D2> RequestHandlerDecorator<In> for RequestHandlerDecoratorComposer<D1, D2>
where
    In: RequestHandler,
    D1: RequestHandlerDecorator<In>,
    D2: RequestHandlerDecorator<D1::Output>,
{
    type Output = D2::Output;
    fn decorate(&self, handler: In) -> Self::Output {
        let output_1 = self.decorator_1.decorate(handler);
        self.decorator_2.decorate(output_1)
    }
}
