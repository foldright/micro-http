use crate::protocol::{Message, ParseError, PayloadSize, RequestHeader};
use bytes::Bytes;
use futures::Stream;
use http_body::{Body, Frame};
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct ReqBody<'conn, S> {
    pub stream: &'conn mut S,
}

impl<'conn, S> Body for ReqBody<'conn, S>
where
    S: Stream<Item = Result<Message<(RequestHeader, PayloadSize)>, ParseError>> + Unpin,
{
    type Data = Bytes;
    type Error = ();

    fn poll_frame(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        todo!()
    }
}
