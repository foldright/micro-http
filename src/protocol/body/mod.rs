mod body;

pub use body::ReqBody;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};



pub enum BodyError {

}

pub struct Body<S: Stream> {
    stream: S,
}

impl<S> Stream for Body<S>
where
    S: Stream<Item = Result<Bytes, BodyError>> + Unpin,
{
    type Item = Result<Bytes, BodyError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.stream.poll_next_unpin(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}
