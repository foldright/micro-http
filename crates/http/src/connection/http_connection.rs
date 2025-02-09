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
use tokio::select;

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
                Some(Ok(Message::Header(header))) => {
                    self.do_process(header, &mut handler).await?;
                }

                Some(Ok(Message::Payload(_))) => {
                    error!("error status because chunked has read in do_process");
                    let error_response = build_error_response(StatusCode::BAD_REQUEST);
                    self.do_send_response(error_response).await?;
                    return Err(ParseError::invalid_body("need header while receive body").into());
                }

                Some(Err(e)) => {
                    error!("can't receive next request, cause {}", e);
                    let error_response = build_error_response(StatusCode::BAD_REQUEST);
                    self.do_send_response(error_response).await?;
                    return Err(e.into());
                }

                None => {
                    info!("cant read more request, break this connection down");
                    return Ok(());
                }
            }
        }
    }

    async fn do_process<H>(&mut self, header: RequestHeader, handler: &mut Arc<H>) -> Result<(), HttpError>
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

        let (req_body, mut body_sender) = ReqBody::body_channel(&mut self.framed_read);

        let request = header.body(req_body);

        // This block handles concurrent processing of the request handler and request body streaming.
        // We need this concurrent processing because:
        // 1. The request handler may not read the entire body, but we still need to drain the body
        //    from the underlying TCP stream to maintain protocol correctness
        // 2. The request handler and body streaming need to happen simultaneously to avoid deadlocks,
        //    since the handler may be waiting for body data while the body sender is waiting to send
        let response_result = {
            // Pin both futures to the stack since they are used in select! macro
            // The futures are lazy and won't start executing until polled
            tokio::pin! {
                let request_handle_future = handler.call(request);
                let body_sender_future = body_sender.send_body();
            }

            // Store the handler result to return after body is fully processed
            #[allow(unused_assignments)]
            let mut result = Option::<Result<_, _>>::None;

            // Keep processing until handler completes
            loop {
                select! {
                    // biased ensures we prioritize handling the response
                    biased;
                    // When handler completes, store result and break
                    response = &mut request_handle_future => {
                        result = Some(response);
                        break;
                    }
                    // Keep processing body chunks in background
                    _ = &mut body_sender_future => {
                        // No action needed - just keep streaming body
                    }
                }
            }
            // Safe: result is Some if handler completed
            result.unwrap()
        };

        // skip body if request handler don't read body
        body_sender.skip_body().await;

        self.send_response(response_result).await?;

        Ok(())
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
