use crate::protocol::{Message, ParseError, PayloadItem, PayloadSize, RequestHeader};
use bytes::Bytes;
use futures::{SinkExt, Stream, StreamExt, channel::mpsc};
use http_body::{Body, Frame, SizeHint};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tracing::error;

pub(crate) fn create_body_sender_receiver<S>(body_stream: &mut S, payload_size: PayloadSize) -> (BodySender<S>, BodyReceiver)
where
    S: Stream<Item = Result<Message<(RequestHeader, PayloadSize)>, ParseError>> + Unpin,
{
    let (signal_sender, signal_receiver) = mpsc::channel(8);
    let (data_sender, data_receiver) = mpsc::channel(8);

    (BodySender::new(body_stream, signal_receiver, data_sender), BodyReceiver::new(signal_sender, data_receiver, payload_size))
}

pub(crate) enum BodyRequestSignal {
    RequestData,
    #[allow(dead_code)]
    Enough,
}

pub(crate) struct BodySender<'conn, S> {
    payload_stream: &'conn mut S,
    signal_receiver: mpsc::Receiver<BodyRequestSignal>,
    data_sender: mpsc::Sender<Result<PayloadItem, ParseError>>,
    eof: bool,
}

impl<'conn, S> BodySender<'conn, S>
where
    S: Stream<Item = Result<Message<(RequestHeader, PayloadSize)>, ParseError>> + Unpin,
{
    pub fn new(
        payload_stream: &'conn mut S,
        signal_receiver: mpsc::Receiver<BodyRequestSignal>,
        data_sender: mpsc::Sender<Result<PayloadItem, ParseError>>,
    ) -> Self {
        Self { payload_stream, signal_receiver, data_sender, eof: false }
    }

    pub(crate) async fn start(&mut self) -> Result<(), ParseError> {
        if self.eof {
            return Ok(());
        }

        while let Some(signal) = self.signal_receiver.next().await {
            match signal {
                BodyRequestSignal::RequestData => match self.read_data().await {
                    Ok(payload_item) => {
                        self.eof = payload_item.is_eof();
                        if let Err(e) = self.data_sender.send(Ok(payload_item)).await {
                            error!("failed to send payload body through channel, {}", e);
                            return Err(ParseError::invalid_body("send body data error"));
                        }

                        if self.eof {
                            return Ok(());
                        }
                    }

                    Err(e) => {
                        error!("failed to read data from body stream, {}", e);
                        if let Err(send_error) = self.data_sender.send(Err(e)).await {
                            error!("failed to send error through channel, {}", send_error);
                            return Err(ParseError::invalid_body("failed to send error through channel"));
                        }
                        break;
                    }
                },

                BodyRequestSignal::Enough => {
                    break;
                }
            }
        }

        self.skip_data().await
    }

    pub(crate) async fn read_data(&mut self) -> Result<PayloadItem, ParseError> {
        match self.payload_stream.next().await {
            Some(Ok(Message::Payload(payload_item))) => Ok(payload_item),
            Some(Ok(Message::Header(_))) => {
                error!("should not receive header in BodySender");
                Err(ParseError::invalid_body("should not receive header in BodySender"))
            }
            Some(Err(e)) => Err(e),
            None => {
                error!("should not receive None in BodySender");
                Err(ParseError::invalid_body("should not receive None in BodySender"))
            }
        }
    }

    pub(crate) async fn skip_data(&mut self) -> Result<(), ParseError> {
        if self.eof {
            return Ok(());
        }

        loop {
            match self.read_data().await {
                Ok(payload_item) if payload_item.is_eof() => {
                    self.eof = true;
                    return Ok(());
                }
                Ok(_payload_item) => {
                    // drop payload_item
                }
                Err(e) => return Err(e),
            }
        }
    }
}

pub(crate) struct BodyReceiver {
    signal_sender: mpsc::Sender<BodyRequestSignal>,
    data_receiver: mpsc::Receiver<Result<PayloadItem, ParseError>>,
    payload_size: PayloadSize,
}

impl BodyReceiver {
    pub(crate) fn new(
        signal_sender: mpsc::Sender<BodyRequestSignal>,
        data_receiver: mpsc::Receiver<Result<PayloadItem, ParseError>>,
        payload_size: PayloadSize,
    ) -> Self {
        Self { signal_sender, data_receiver, payload_size }
    }
}

impl BodyReceiver {
    pub async fn receive_data(&mut self) -> Result<PayloadItem, ParseError> {
        if let Err(e) = self.signal_sender.send(BodyRequestSignal::RequestData).await {
            error!("failed to send request_more through channel, {}", e);
            return Err(ParseError::invalid_body("failed to send signal when receive body data"));
        }

        self.data_receiver
            .next()
            .await
            .unwrap_or_else(|| Err(ParseError::invalid_body("body stream should not receive None when receive data")))
    }
}

impl Body for BodyReceiver {
    type Data = Bytes;
    type Error = ParseError;

    fn poll_frame(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();

        tokio::pin! {
            let future = this.receive_data();
        }

        match future.poll(cx) {
            Poll::Ready(Ok(PayloadItem::Chunk(bytes))) => Poll::Ready(Some(Ok(Frame::data(bytes)))),
            Poll::Ready(Ok(PayloadItem::Eof)) => Poll::Ready(None),
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
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
