use bytes::Bytes;
use std::error::Error;
use std::fmt::{Debug, Display};

use futures::StreamExt;
use http::header::EXPECT;
use http::{Response, StatusCode};
use http_body::Body;
use http_body_util::{BodyExt, Empty};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

use crate::codec::RequestDecoder;
use crate::handler::Handler;
use crate::protocol::body::ReqBody;
use crate::protocol::{HttpError, Message, ParseError, PayloadItem, PayloadSize, RequestHeader, ResponseHead, SendError};

use crate::connection::message_writer::MessageWriter;
use tokio_util::codec::FramedRead;
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
#[derive(Debug)]
pub struct HttpConnection<R, W>
where
    R: Debug,
    W: Debug,
{
    framed_read: Option<FramedRead<R, RequestDecoder>>,
    message_writer: MessageWriter<W>,
}

impl<R, W> HttpConnection<R, W>
where
    R: AsyncRead + Unpin + Send + Debug,
    W: AsyncWrite + Unpin + Debug,
{
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            framed_read: Some(FramedRead::with_capacity(reader, RequestDecoder::new(), 8 * 1024)),
            message_writer: MessageWriter::with_capacity(writer, 8 * 1024),
        }
    }

    pub async fn process<H>(mut self, handler: &H) -> Result<(), HttpError>
    where
        H: Handler,
        H::RespBody: Body<Data = Bytes> + Unpin,
        <H::RespBody as Body>::Error: Display,
    {
        loop {
            let framed_read = self.framed_read.as_mut().expect("framed reader must be available while processing requests");

            match framed_read.next().await {
                Some(Ok(Message::Header((header, payload_size)))) => {
                    self.do_process(header, payload_size, handler).await?;
                }

                Some(Ok(Message::Payload(PayloadItem::Eof))) => continue,

                Some(Ok(Message::Payload(_))) => {
                    error!("error status because chunked has read in do_process");
                    let error_response = build_error_response(StatusCode::BAD_REQUEST);
                    self.do_send_response(error_response).await?;
                    return Err(ParseError::invalid_body("need header while receive body").into());
                }

                Some(Err(ParseError::Io { source })) => {
                    info!("connection io error: {}", source);
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

    async fn do_process<H>(&mut self, header: RequestHeader, payload_size: PayloadSize, handler: &H) -> Result<(), HttpError>
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
                let writer = self.message_writer.get_mut();
                // Send a "100 Continue" response to the client.
                let _ = writer.write(b"HTTP/1.1 100 Continue\r\n\r\n").await.map_err(SendError::io)?;
                writer.flush().await.map_err(SendError::io)?;
                // Log the event of sending a "100 Continue" response.
                info!("receive expect request header, sent continue response");
            }
        }

        let framed_read = self.framed_read.take().expect("framed reader must exist when creating request body");
        let (req_body, req_body_state) = ReqBody::create_req_body(framed_read, payload_size);
        let request = header.body(req_body);

        let response_result = handler.call(request).await;

        let framed_read = req_body_state.finish().await?;
        self.framed_read = Some(framed_read);

        self.send_response(response_result).await
    }

    async fn send_response<T, E>(&mut self, response_result: Result<Response<T>, E>) -> Result<(), HttpError>
    where
        T: Body<Data = Bytes> + Unpin,
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
        T: Body<Data = Bytes> + Unpin,
        T::Error: Display,
    {
        let (header_parts, mut body) = response.into_parts();
        let payload_size: PayloadSize = body.size_hint().into();
        let response_head = ResponseHead::from_parts(header_parts, ());
        match payload_size {
            PayloadSize::Empty => {
                self.message_writer.send_head(response_head, payload_size).await?;
                Ok(())
            }
            PayloadSize::Length(_) => {
                match body.frame().await {
                    Some(Ok(frame)) => {
                        let first = frame.into_data().map_err(|_e| SendError::invalid_body("resolve body response error"))?;
                        self.message_writer.send_head_and_body(response_head, payload_size, first.as_ref()).await?;
                    }
                    Some(Err(e)) => return Err(SendError::invalid_body(format!("resolve response body error: {e}")).into()),
                    None => {
                        return Err(SendError::invalid_body("response body ended before its declared content-length").into());
                    }
                }

                loop {
                    match body.frame().await {
                        Some(Ok(frame)) => {
                            let payload = frame.into_data().map_err(|_e| SendError::invalid_body("resolve body response error"))?;
                            self.message_writer.send_body(payload.as_ref()).await?;
                        }
                        Some(Err(e)) => return Err(SendError::invalid_body(format!("resolve response body error: {e}")).into()),
                        None => {
                            self.message_writer.finish_body()?;
                            return Ok(());
                        }
                    }
                }
            }
            PayloadSize::Chunked => {
                match body.frame().await {
                    Some(Ok(frame)) => {
                        let first = frame.into_data().map_err(|_e| SendError::invalid_body("resolve body response error"))?;
                        self.message_writer.send_head_and_chunk(response_head, first.as_ref()).await?;
                    }
                    Some(Err(e)) => return Err(SendError::invalid_body(format!("resolve response body error: {e}")).into()),
                    None => {
                        self.message_writer.send_head(response_head, payload_size).await?;
                        self.message_writer.send_chunk_eof().await?;
                        return Ok(());
                    }
                }

                loop {
                    match body.frame().await {
                        Some(Ok(frame)) => {
                            let payload = frame.into_data().map_err(|_e| SendError::invalid_body("resolve body response error"))?;
                            self.message_writer.send_chunk(payload.as_ref()).await?;
                        }
                        Some(Err(e)) => return Err(SendError::invalid_body(format!("resolve response body error: {e}")).into()),
                        None => {
                            self.message_writer.send_chunk_eof().await?;
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
}

fn build_error_response(status_code: StatusCode) -> Response<Empty<Bytes>> {
    Response::builder().status(status_code).body(Empty::<Bytes>::new()).unwrap()
}
