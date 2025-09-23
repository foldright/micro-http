use std::error::Error;
use std::fmt::Display;
use bytes::Bytes;
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use http::header::EXPECT;
use http::{Response, StatusCode};
use http_body::Body;
use http_body_util::{BodyExt, Empty};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

use crate::codec::{RequestDecoder, ResponseEncoder};
use crate::handler::Handler;
use crate::protocol::body::ReqBody;
use crate::protocol::{HttpError, Message, ParseError, PayloadItem, PayloadSize, RequestHeader, ResponseHead, SendError};

use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{error, info};

/// An HTTP connection that manages request processing and response streaming
///
/// `HttpConnection` handles the full lifecycle of an HTTP connection, including:
/// - Reading and decoding requests
/// - Processing request headers and bodies
/// - Handling expect-continue mechanism
/// - Streaming responses back to clients
///
/// # Type Parameters
///
/// * `R`: The async readable stream type
/// * `W`: The async writable stream type
///
pub struct HttpConnection<R, W> {
    framed_read: FramedRead<R, RequestDecoder>,
    framed_write: FramedWrite<W, ResponseEncoder>,
}

impl<R, W> HttpConnection<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            framed_read: FramedRead::with_capacity(reader, RequestDecoder::new(), 8 * 1024),
            framed_write: FramedWrite::new(writer, ResponseEncoder::new()),
        }
    }

    pub async fn process<H>(mut self, mut handler: Arc<H>) -> Result<(), HttpError>
    where
        H: Handler,
        H::RespBody: Body<Data = Bytes> + Unpin,
        <H::RespBody as Body>::Error: Display,
    {
        loop {
            match self.framed_read.next().await {
                Some(Ok(Message::Header((header, payload_size)))) => {
                    self.do_process(header, payload_size, &mut handler).await?;
                }

                Some(Ok(Message::Payload(PayloadItem::Eof))) => continue,

                Some(Ok(Message::Payload(_))) => {
                    error!("error status because chunked has read in do_process");
                    let error_response = build_error_response(StatusCode::BAD_REQUEST);
                    self.do_send_response(error_response).await?;
                    return Err(ParseError::invalid_body("need header while receive body").into());
                }

                Some(Err(ParseError::Io { source})) => {
                    info!("connection io error: {}, remote client: {}", source, );
                    return Ok(());
                }

                Some(Err(e)) => {
                    error!("can't receive next request, cause {}", e);
                    return Err(e.into());
                }

                None => {
                    info!("can't read more request, break this connection down");
                    return Ok(());
                }
            }
        }
    }

    async fn do_process<H>(&mut self, header: RequestHeader, payload_size: PayloadSize, handler: &mut Arc<H>) -> Result<(), HttpError>
    where
        H: Handler,
        H::RespBody: Body<Data = Bytes> + Unpin,
        <H::RespBody as Body>::Error: Display,
    {
        // Check if the request header contains the "Expect: 100-continue" field.
        if let Some(value) = header.headers().get(EXPECT) {
            let slice = value.as_bytes();
            // Verify if the value of the "Expect" field is "100-continue".
            if slice.len() >= 4 && &slice[0..4] == b"100-" {
                let writer = self.framed_write.get_mut();
                // Send a "100 Continue" response to the client.
                let _ = writer.write(b"HTTP/1.1 100 Continue\r\n\r\n").await.map_err(SendError::io)?;
                writer.flush().await.map_err(SendError::io)?;
                // Log the event of sending a "100 Continue" response.
                info!("receive expect request header, sent continue response");
            }
        }

        let (req_body, maybe_body_sender) = ReqBody::create_req_body(&mut self.framed_read, payload_size);
        let request = header.body(req_body);

        let response_result = match maybe_body_sender {
            None => handler.call(request).await,
            Some(mut body_sender) => {
                let (handler_result, body_send_result) = tokio::join!(handler.call(request), body_sender.start());

                // check if body sender has error
                body_send_result?;
                handler_result
            }
        };

        self.send_response(response_result).await
    }

    async fn send_response<T, E>(&mut self, response_result: Result<Response<T>, E>) -> Result<(), HttpError>
    where
        T: Body + Unpin,
        T::Error: Display,
        E: Into<Box<dyn Error + Send + Sync>>,
    {
        match response_result {
            Ok(response) => self.do_send_response(response).await,
            Err(e) => {
                error!("handle response error, cause: {}", e.into());
                let error_response = build_error_response(StatusCode::INTERNAL_SERVER_ERROR);
                self.do_send_response(error_response).await
            }
        }
    }

    async fn do_send_response<T>(&mut self, response: Response<T>) -> Result<(), HttpError>
    where
        T: Body + Unpin,
        T::Error: Display,
    {
        let (header_parts, mut body) = response.into_parts();

        let payload_size = {
            let size_hint = body.size_hint();
            match size_hint.exact() {
                Some(0) => PayloadSize::Empty,
                Some(length) => PayloadSize::Length(length),
                None => PayloadSize::Chunked,
            }
        };

        let header = Message::<_, T::Data>::Header((ResponseHead::from_parts(header_parts, ()), payload_size));
        if !payload_size.is_empty() {
            self.framed_write.feed(header).await?;
        } else {
            // using send instead of feed, because we want to flush the underlying IO
            // when response only has header, we need to send header,
            // otherwise, we just feed header to the buffer
            self.framed_write.send(header).await?;
        }

        loop {
            match body.frame().await {
                Some(Ok(frame)) => {
                    let payload_item =
                        frame.into_data().map(PayloadItem::Chunk).map_err(|_e| SendError::invalid_body("resolve body response error"))?;

                    self.framed_write
                        .send(Message::Payload(payload_item))
                        .await
                        .map_err(|_e| SendError::invalid_body("can't send response"))?;
                }
                Some(Err(e)) => return Err(SendError::invalid_body(format!("resolve response body error: {e}")).into()),
                None => {
                    self.framed_write
                        // using feed instead of send, because we don't want to flush the underlying IO
                        .feed(Message::Payload(PayloadItem::<T::Data>::Eof))
                        .await
                        .map_err(|e| SendError::invalid_body(format!("can't send eof response: {}", e)))?;
                    return Ok(());
                }
            }
        }
    }
}

fn build_error_response(status_code: StatusCode) -> Response<Empty<Bytes>> {
    Response::builder().status(status_code).body(Empty::<Bytes>::new()).unwrap()
}
