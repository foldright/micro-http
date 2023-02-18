use std::pin::Pin;
use std::task::{ready, Context, Poll};

use bytes::Bytes;

use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, SinkExt, Stream, StreamExt};

use http_body::{Body, Frame};
use tracing::{error, info};

use crate::protocol::{Message, ParseError, PayloadItem, RequestHeader};

pub struct ReqBody {
    signal: mpsc::Sender<oneshot::Sender<PayloadItem>>,
    receiving: Option<oneshot::Receiver<PayloadItem>>,
}

impl ReqBody {
    fn new(signal: mpsc::Sender<oneshot::Sender<PayloadItem>>) -> Self {
        Self { signal, receiving: None }
    }

    pub fn body_channel<S>(payload_stream: &mut S) -> (ReqBody, ReqBodySender<S>)
    where
        S: Stream + Unpin,
    {
        let (tx, receiver) = mpsc::channel(16);

        let req_body = ReqBody::new(tx);

        let body_sender = ReqBodySender { payload_stream, receiver, eof: false };

        (req_body, body_sender)
    }
}

pub struct ReqBodySender<'conn, S>
where
    S: Stream + Unpin,
{
    payload_stream: &'conn mut S,
    receiver: mpsc::Receiver<oneshot::Sender<PayloadItem>>,
    eof: bool,
}

impl<'conn, S> ReqBodySender<'conn, S>
where
    S: Stream<Item = Result<Message<RequestHeader>, ParseError>> + Unpin,
{
    pub async fn send_body(&mut self) -> Result<(), ParseError> {
        loop {
            if self.eof {
                return Ok(());
            }

            if let Some(sender) = self.receiver.next().await {
                match self.payload_stream.next().await {
                    Some(Ok(Message::Payload(payload_item))) => {
                        if payload_item.is_eof() {
                            self.eof = true;
                        }
                        sender.send(payload_item).unwrap();
                    }

                    Some(Ok(Message::Header(_header))) => {
                        error!("received header from receive body phase");
                        return Err(ParseError::InvalidBody { reason: "received header from receive body phase".into() });
                    }

                    Some(Err(e)) => {
                        return Err(e);
                    }

                    None => {
                        error!("cant read body");
                        return Err(ParseError::InvalidBody { reason: "cant read body".into() });
                    }
                }
            }
        }
    }

    pub async fn skip_body(&mut self) {
        if !self.eof {
            let mut size: usize = 0;
            while let Some(Ok(Message::Payload(payload_item))) = self.payload_stream.next().await {
                if payload_item.is_eof() {
                    self.eof = true;
                    if size > 0 {
                        info!(size = size, "skip request body");
                    }
                    break;
                }

                if let Some(bytes) = payload_item.as_bytes() {
                    size += bytes.len();
                }
            }
        }
    }
}

impl Body for ReqBody {
    type Data = Bytes;
    type Error = ParseError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        loop {
            if let Some(oneshot_receiver) = &mut self.receiving {
                return match ready!(oneshot_receiver.poll_unpin(cx)) {
                    Ok(PayloadItem::Chunk(bytes)) => {
                        self.receiving.take();
                        Poll::Ready(Some(Ok(Frame::data(bytes))))
                    }
                    Ok(PayloadItem::Eof) => {
                        self.receiving.take();
                        Poll::Ready(None)
                    }
                    Err(_) => {
                        self.receiving.take();
                        Poll::Ready(Some(Err(ParseError::InvalidBody { reason: "parse body canceled".into() })))
                    }
                };
            }

            match ready!(self.signal.poll_ready_unpin(cx)) {
                Ok(_) => {
                    let (tx, rx) = oneshot::channel();
                    match self.signal.start_send(tx) {
                        Ok(_) => {
                            self.receiving = Some(rx);
                            continue;
                        }
                        Err(e) => return Poll::Ready(Some(Err(ParseError::InvalidBody { reason: e.to_string() }))),
                    }
                }
                Err(e) => return Poll::Ready(Some(Err(ParseError::InvalidBody { reason: e.to_string() }))),
            };
        }
    }
}
