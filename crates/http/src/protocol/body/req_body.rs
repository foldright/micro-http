use std::pin::Pin;
use std::task::{ready, Context, Poll};

use bytes::Bytes;

use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, SinkExt, Stream, StreamExt};

use http_body::{Body, Frame};
use tracing::{error, info};

use crate::protocol::{Message, ParseError, PayloadItem, RequestHeader};

/// ReqBody implements an asynchronous streaming mechanism for HTTP request bodies.
///
/// # Design Goals
///
/// The main design goals of ReqBody are:
/// 1. Provide efficient streaming of request bodies without buffering entire payload in memory
/// 2. Bridge the gap between low-level payload streams and high-level http_body::Body interface
/// 3. Support concurrent processing of request handling and body streaming
/// 4. Allow proper cleanup of unread body data to maintain protocol correctness
///
/// # Architecture
///
/// ReqBody uses a channel-based architecture:
/// - `ReqBody`: Consumer side that implements http_body::Body
/// - `ReqBodySender`: Producer side that reads from raw payload stream
/// - They communicate through a mpsc channel and oneshot channels
///
/// # Example Flow
///
/// 1. HttpConnection creates ReqBody/ReqBodySender pair
/// 2. ReqBody is passed to request handler for body consumption
/// 3. ReqBodySender runs concurrently to stream payload chunks
/// 4. If handler doesn't read entire body, remaining data is skipped
pub struct ReqBody {
    signal: mpsc::Sender<oneshot::Sender<PayloadItem>>,
    receiving: Option<oneshot::Receiver<PayloadItem>>,
}

impl ReqBody {
    /// Creates a new ReqBody with a channel for signaling payload requests.
    ///
    /// The signal sender is used to request new payload chunks from the producer side.
    fn new(signal: mpsc::Sender<oneshot::Sender<PayloadItem>>) -> Self {
        Self { signal, receiving: None }
    }

    /// Creates a body streaming channel pair for processing HTTP request bodies.
    ///
    /// This is the main entry point for setting up request body streaming. It creates
    /// the necessary channels and returns both consumer and producer components.
    ///
    /// The returned ReqBody implements http_body::Body and can be passed to request handlers,
    /// while ReqBodySender handles reading from the underlying stream.
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

/// ReqBodySender handles reading body chunks from the raw payload stream.
///
/// This component runs concurrently with request processing to ensure:
/// 1. Body chunks are available when the handler needs them
/// 2. All body data is properly drained from the connection
/// 3. Resources are cleaned up appropriately
///
/// The sender maintains an EOF flag to track when the complete body has been read,
/// which is crucial for proper connection handling.
pub struct ReqBodySender<'conn, S>
where
    S: Stream + Unpin,
{
    payload_stream: &'conn mut S,
    receiver: mpsc::Receiver<oneshot::Sender<PayloadItem>>,
    eof: bool,
}

impl<S> ReqBodySender<'_, S>
where
    S: Stream<Item = Result<Message<RequestHeader>, ParseError>> + Unpin,
{
    /// Streams body chunks from payload stream to ReqBody consumer.
    ///
    /// This method runs in a loop, responding to chunk requests from the ReqBody
    /// until either:
    /// - The complete body is streamed (EOF)
    /// - An error occurs during streaming
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
                        return Err(ParseError::invalid_body("received header from receive body phase"));
                    }

                    Some(Err(e)) => {
                        return Err(e);
                    }

                    None => {
                        error!("cant read body");
                        return Err(ParseError::invalid_body("cant read body"));
                    }
                }
            }
        }
    }

    /// Drains any remaining body chunks from the payload stream.
    ///
    /// This is critical for maintaining HTTP protocol correctness when:
    /// - The handler doesn't read the complete body
    /// - The connection will be reused for future requests
    ///
    /// It ensures the connection is in a clean state for the next request.
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

/// Implements standard HTTP body interface for request bodies.
///
/// This implementation bridges our custom streaming mechanism with the standard
/// http_body::Body trait, allowing ReqBody to work seamlessly with HTTP
/// handlers and middleware that expect the standard interface.
impl Body for ReqBody {
    type Data = Bytes;
    type Error = ParseError;

    fn poll_frame(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
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
                        Poll::Ready(Some(Err(ParseError::invalid_body("parse body canceled"))))
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
                        Err(e) => return Poll::Ready(Some(Err(ParseError::invalid_body(e)))),
                    }
                }
                Err(e) => return Poll::Ready(Some(Err(ParseError::invalid_body(e)))),
            };
        }
    }
}
