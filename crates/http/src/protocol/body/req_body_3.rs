use crate::protocol::{ParseError, PayloadItem, PayloadSize};
use bytes::{Buf, Bytes};
use futures::{Stream, StreamExt};
use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::channel::mpsc::Sender;

pub struct RequestBody<S> {
    stream: S,
    payload_size: PayloadSize,
    end: bool,
}

impl<S> RequestBody<S> {
    pub(crate) fn new(stream: S, payload_size: PayloadSize) -> Self {
        Self { stream, payload_size, end: false }
    }
}

impl<S> Body for RequestBody<S>
where
    S: Stream<Item = PayloadItem> + Unpin,
{
    type Data = Bytes;
    type Error = ParseError;

    fn poll_frame(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();
        match this.stream.poll_next_unpin(cx) {
            Poll::Ready(Some(PayloadItem::Chunk(data))) => Poll::Ready(Some(Ok(Frame::data(data)))),
            Poll::Ready(Some(PayloadItem::Eof)) => {
                this.end = true;
                Poll::Ready(None)
            }
            // the channel is closed
            Poll::Ready(None) => Poll::Ready(Some(Err(ParseError::invalid_body("received body error")))),
            Poll::Pending => Poll::Pending,
        }
    }

    fn is_end_stream(&self) -> bool {
        self.end
    }

    fn size_hint(&self) -> SizeHint {
        self.payload_size.into()
    }
}

impl From<SizeHint> for PayloadSize {
    fn from(size_hint: SizeHint) -> Self {
        match size_hint.exact() {
            Some(0) => PayloadSize::new_empty(),
            Some(length) => PayloadSize::new_length(length),
            None => PayloadSize::new_chunked(),
        }
    }
}

impl From<PayloadSize> for SizeHint {
    fn from(payload_size: PayloadSize) -> Self {
        match payload_size {
            PayloadSize::Length(length) => SizeHint::with_exact(length),
            PayloadSize::Chunked => SizeHint::new(),
            PayloadSize::Empty => SizeHint::with_exact(0),
        }
    }
}

