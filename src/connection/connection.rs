use std::error::Error;

use bytes::Bytes;
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use http::{Response, StatusCode};
use http_body::Body;
use http_body_util::{BodyExt, Empty};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::select;

use crate::codec::{DecodeError, RequestDecoder, ResponseEncoder};
use crate::handler::Handler;
use crate::protocol::body::ReqBody;
use crate::protocol::{HttpError, Message, PayloadItem, PayloadSize, RequestHeader, ResponseHead};

use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{error, info};

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
                    return Err(DecodeError::Body { message: "need header while receive body".into() }.into());
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
    {
        let (req_body, mut body_sender) = ReqBody::body_channel(&mut self.framed_read);

        let request = header.body(req_body);

        // concurrent compute the request handler and the body sender
        let response_result = {
            // both are lazy, and need to pin in the stack while using select!
            tokio::pin! {
                let request_handle_future = handler.handle(request);
                let body_sender_future = body_sender.send_body();
            }

            let mut result = Option::<Result<_, _>>::None;
            loop {
                select! {
                    biased;
                    response = &mut request_handle_future => {
                        result = Some(response);
                        break;
                    }
                    _ = &mut body_sender_future => {
                        //no op
                    }
                }
            }
            result.unwrap()
        };

        // skip body if request handler don't read body
        body_sender.skip_body().await;

        self.send_response(response_result).await?;

        Ok(())
    }

    async fn send_response<T, E>(&mut self, response_result: Result<Response<T>, E>) -> Result<(), HttpError>
    where
        T: Body<Data = Bytes> + Unpin,
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
    {
        let (header_parts, mut body) = response.into_parts();

        let payload_size = {
            let size_hint = body.size_hint();
            match size_hint.exact() {
                Some(0) => PayloadSize::Empty,
                Some(length) => PayloadSize::Length(length as usize),
                None => PayloadSize::Chunked,
            }
        };

        self.framed_write.send(Message::Header((ResponseHead::from_parts(header_parts, ()), payload_size))).await?;

        loop {
            match body.frame().await {
                Some(Ok(frame)) => {
                    let payload_item = frame
                        .into_data()
                        .map(|bytes| PayloadItem::Chunk(bytes))
                        .map_err(|_e| DecodeError::Body { message: "resolve body response error".into() })?;

                    self.framed_write
                        .send(Message::Payload(payload_item))
                        .await
                        .map_err(|_e| DecodeError::Body { message: "can't send response".into() })?;
                }
                Some(Err(_e)) => return Err(DecodeError::Body { message: "resolve response body error".into() }.into()),
                None => return Ok(()),
            }
        }
    }
}

fn build_error_response(status_code: StatusCode) -> Response<Empty<Bytes>> {
    Response::builder().status(status_code).body(Empty::<Bytes>::new()).unwrap()
}
