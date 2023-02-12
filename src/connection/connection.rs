use std::error::Error;

use bytes::{BufMut, Bytes};
use std::sync::Arc;

use futures::channel::mpsc::Receiver;
use futures::channel::{mpsc, oneshot};
use futures::{join, SinkExt, StreamExt};
use http::{Response, StatusCode};
use http_body::Body;
use http_body_util::{BodyExt, Empty};

use crate::codec::{HeaderEncoder, Message, ParseError, RequestDecoder};
use crate::handler::Handler;
use crate::protocol::body::ReqBody;
use crate::protocol::RequestHeader;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{error, info};

pub struct HttpConnection {
    framed_read: FramedRead<OwnedReadHalf, RequestDecoder>,
    framed_write: FramedWrite<OwnedWriteHalf, HeaderEncoder>,
}

impl HttpConnection {
    pub fn new(stream: TcpStream) -> Self {
        let (reader, writer) = stream.into_split();
        Self {
            framed_read: FramedRead::with_capacity(reader, RequestDecoder::new(), 8 * 1024),
            framed_write: FramedWrite::new(writer, HeaderEncoder),
        }
    }

    async fn do_process<H>(&mut self, header: RequestHeader, handler: &mut Arc<H>) -> Result<(), ParseError>
    where
        H: Handler,
        H::RespBody: Body + Unpin,
    {
        let (tx, rx) = mpsc::channel(1024);

        let req_body = ReqBody::new(tx);
        let request = header.body(req_body);

        let processing = handler.handle(request);
        let body_sender = self.send_body(rx);

        let (response_result, sender_result) = join!(processing, body_sender);

        self.send_response(response_result).await?;

        // try to consume left body if need
        let eof = sender_result?;
        if !eof {
            while let Some(Ok(Message::Chunked(some_bytes))) = self.framed_read.next().await {
                if some_bytes.is_none() {}
                break;
            }
        }

        Ok(())
    }

    async fn send_body(&mut self, mut rx: Receiver<oneshot::Sender<Option<Bytes>>>) -> Result<bool, ParseError> {
        let mut eof = false;
        loop {
            if eof {
                return Ok(true);
            }

            if let Some(sender) = rx.next().await {
                match self.framed_read.next().await {
                    Some(Ok(Message::Chunked(some_bytes))) => {
                        if some_bytes.is_none() {
                            eof = true;
                        }
                        sender.send(some_bytes).unwrap();
                    }

                    Some(Ok(Message::Header(_header))) => {
                        error!("received header from receive body phase");
                        return Err(ParseError::Body { message: "received header from receive body phase".into() });
                    }

                    Some(Err(e)) => {
                        return Err(e);
                    }

                    None => {
                        error!("cant read body");
                        return Err(ParseError::Body { message: "cant read body".into() });
                    }
                }
            }
        }
    }

    pub async fn process<H>(mut self, mut handler: Arc<H>) -> Result<(), ParseError>
    where
        H: Handler,
        H::RespBody: Body + Unpin,
    {
        loop {
            match self.framed_read.next().await {
                Some(Ok(Message::Header(header))) => {
                    self.do_process(header, &mut handler).await?;
                }

                Some(Ok(Message::Chunked(_))) => {
                    // not exactly, request can read part body
                    unreachable!("can't reach here because chunked has read in do_process");
                }

                Some(Err(_e)) => {
                    todo!("convert parseError to response");
                    break;
                }

                None => {
                    // todo: add remote addr to log
                    info!("cant read more request, break this connection down");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn send_response<T, E>(&mut self, response_result: Result<Response<T>, E>) -> Result<(), ParseError>
    where
        T: Body + Unpin,
        E: Into<Box<dyn Error + Send + 'static>>,
    {
        match response_result {
            Ok(response) => self.do_send_response(response).await,
            Err(_) => {
                let error_response =
                    Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Empty::<Bytes>::new()).unwrap();

                self.do_send_response(error_response).await
            }
        }
    }

    async fn do_send_response<T>(&mut self, response: Response<T>) -> Result<(), ParseError>
    where
        T: Body + Unpin,
    {
        let (parts, mut body) = response.into_parts();
        self.framed_write.send(parts).await.map_err(|_e| ParseError::Body { message: "can't send response".into() })?;

        loop {
            match body.frame().await {
                Some(Ok(frame)) => {
                    let data = frame
                        .into_data()
                        .map_err(|_e| ParseError::Body { message: "resolve body response error".into() })?;
                    self.framed_write.write_buffer_mut().put(data);
                    self.framed_write
                        .flush()
                        .await
                        .map_err(|_e| ParseError::Body { message: "can't flush response".into() })?;
                }
                Some(Err(_e)) => return Err(ParseError::Body { message: "resolve response body error".into() }),
                None => return Ok(()),
            }
        }
    }
}
