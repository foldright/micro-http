use crate::protocol::body::body_channel::{create_body_sender_receiver, BodyReceiver, BodySender};
use crate::protocol::{Message, ParseError, PayloadSize, RequestHeader};
use bytes::Bytes;
use futures::Stream;
use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

pub enum ReqBody {
    Receiver(BodyReceiver),
    NoBody,
}

impl ReqBody {
    pub(crate) fn create_req_body<S>(body_stream: &mut S, payload_size: PayloadSize) -> (ReqBody, Option<BodySender<S>>)
    where
        S: Stream<Item = Result<Message<(RequestHeader, PayloadSize)>, ParseError>> + Unpin,
    {
        match payload_size {
            PayloadSize::Empty | PayloadSize::Length(0) => (ReqBody::NoBody, None),
            _ => {
                let (sender, receiver) = create_body_sender_receiver(body_stream, payload_size);
                (ReqBody::Receiver(receiver), Some(sender))
            }
        }
    }
}

impl Body for ReqBody {
    type Data = Bytes;
    type Error = ParseError;

    fn poll_frame(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();
        match this {
            ReqBody::Receiver(body_receiver) => Pin::new(body_receiver).poll_frame(cx),
            ReqBody::NoBody => Poll::Ready(None),
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            ReqBody::NoBody => true,
            ReqBody::Receiver(body_receiver) => body_receiver.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            ReqBody::NoBody => SizeHint::with_exact(0),
            ReqBody::Receiver(body_receiver) => body_receiver.size_hint(),
        }
    }
}
