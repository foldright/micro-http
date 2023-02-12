use std::pin::Pin;
use std::task::{ready, Context, Poll};

use bytes::Bytes;

use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, SinkExt};

use http_body::{Body, Frame};

use crate::codec::ParseError;

pub struct ReqBody {
    signal: mpsc::Sender<oneshot::Sender<Option<Bytes>>>,
    receiving: Option<oneshot::Receiver<Option<Bytes>>>,
}

impl ReqBody {
    pub fn new(signal: mpsc::Sender<oneshot::Sender<Option<Bytes>>>) -> Self {
        Self { signal, receiving: None }
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
                    Ok(Some(bytes)) => {
                        self.receiving.take();
                        Poll::Ready(Some(Ok(Frame::data(bytes))))
                    }
                    Ok(None) => {
                        self.receiving.take();
                        Poll::Ready(None)
                    }
                    Err(_) => {
                        self.receiving.take();
                        Poll::Ready(Some(Err(ParseError::Body { message: "parse body canceled".into() })))
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
                        Err(e) => return Poll::Ready(Some(Err(ParseError::Body { message: e.to_string() }))),
                    }
                }
                Err(e) => return Poll::Ready(Some(Err(ParseError::Body { message: e.to_string() }))),
            };
        }
    }
}
