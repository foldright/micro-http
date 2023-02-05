use std::error::Error;

use std::sync::Arc;
use anyhow::anyhow;
use bytes::{BufMut, Bytes};

use futures::{SinkExt, StreamExt};
use http::{Response, StatusCode};
use http_body::Body;
use http_body_util::{BodyExt, Empty};

use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::info;
use crate::codec::{BodyDecoder, HeaderDecoder, HeaderEncoder};
use crate::handler::Handler;
use crate::protocol::body::ReqBody;

pub struct HttpConnection {
    framed_read: FramedRead<OwnedReadHalf, HeaderDecoder>,
    framed_write: FramedWrite<OwnedWriteHalf, HeaderEncoder>,
}

impl HttpConnection {
    pub fn new(stream: TcpStream) -> Self {
        let (reader, writer) = stream.into_split();
        Self {
            framed_read: FramedRead::with_capacity(reader, HeaderDecoder, 4 * 1024),
            framed_write: FramedWrite::new(writer, HeaderEncoder),
        }
    }

    pub async fn process<H>(mut self, handler: Arc<H>) -> crate::Result<()>
        where H: Handler,
              H::RespBody: Body + Unpin,
    {
        while let Some(Ok(request_header)) = self.framed_read.next().await {
            info!(path = request_header.uri().to_string(), "received request");

            let body_framed = self.framed_read.map_decoder(|_| {
                let body_length = ReqBody::parse_body_length(&request_header);
                BodyDecoder::from(body_length.unwrap())
            });

            let body = ReqBody::new(body_framed);
            let mut request = request_header.body(body);

            let response = handler.handle(&mut request).await;

            let mut body = request.into_body();
            Self::consume_body(&mut body).await?;
            self.framed_read = body.framed_read.map_decoder(|_| HeaderDecoder);

            self.send_response(response).await?;
        }

        Ok(())
    }

    async fn consume_body(body: &mut ReqBody) -> crate::Result<()> {
        loop {
            match body.frame().await {
                None => return Ok(()),
                Some(Ok(_)) => continue,
                Some(Err(_e)) => return Err(anyhow!("consume body error"))
            }
        }
    }

    async fn send_response<T, E>(&mut self, response_result: Result<Response<T>, E>) -> crate::Result<()>
        where T: Body + Unpin,
              E: Into<Box<dyn Error + Send + 'static>>,
    {
        match response_result {
            Ok(response) => self.do_send_response(response).await,
            Err(_) => {
                let error_response = Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Empty::<Bytes>::new())
                    .unwrap();

                self.do_send_response(error_response).await
            }
        }
    }

    async fn do_send_response<T>(&mut self, response: Response<T>) -> crate::Result<()>
        where T: Body + Unpin,
    {
        let (parts, mut body) = response.into_parts();
        self.framed_write.send(parts).await?;

        loop {
            match body.frame().await {
                Some(Ok(frame)) => {
                    let data = frame.into_data()
                        .map_err(|_e| anyhow!("get response data error"))?;
                    self.framed_write.write_buffer_mut().put(data);
                    self.framed_write.flush().await?
                }
                Some(Err(_e)) => return Err(anyhow!("consume body error")),
                None => return Ok(()),
            }
        }
    }
}