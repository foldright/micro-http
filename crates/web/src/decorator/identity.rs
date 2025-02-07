use crate::decorator::Decorator;

#[derive(Default, Clone, Copy, Debug)]
pub struct IdentityDecorator;

impl<In> Decorator<In> for IdentityDecorator {
    type Out = In;
    
    #[inline(always)]
    fn decorate(&self, raw: In) -> Self::Out {
        raw
    }
}