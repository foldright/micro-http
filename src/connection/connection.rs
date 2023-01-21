use std::mem::MaybeUninit;

use bytes::BytesMut;
use http::Request;
use httparse::Status;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{error, info};

use crate::protocol::parse_request_header;

const MAX_HEADER_BYTES: usize = 4 * 1024;

pub struct Connection {
    stream: TcpStream,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(tcp_stream: TcpStream) -> Self {
        Self {
            stream: tcp_stream,
            buffer: BytesMut::with_capacity(1024 * 4),
        }
    }

    async fn shutdown(&mut self) {
        match self.stream.shutdown().await {
            Ok(_) => {}
            Err(e) => {
                error!(e = %e, "shutdown connection error")
            }
        }
    }

    pub async fn read_header<T>(&mut self) -> crate::Result<Request<Option<T>>> {
        loop {
            self.stream.readable().await?;

            match self.stream.read_buf(&mut self.buffer).await? {
                0 => {
                    if self.buffer.is_empty() {
                        continue;
                    } else {
                        return Err("connection reset by peer".into());
                    }
                }
                n => {
                    info!(bytes = %n, "receive: ");
                }
            }

            match Self::parse_header(&mut self.buffer).await? {
                None => {
                    // buffer reach max size
                    if self.buffer.len() >= MAX_HEADER_BYTES {
                        error!(bytes = %self.buffer.len(), "header reached max bytes");
                        self.shutdown().await;
                        return Err("header reached max bytes".into());
                    }

                    continue;
                }
                Some(request) => {
                    return Ok(request);
                }
            }
        }
    }


    async fn parse_header<T>(bytes: &mut BytesMut) -> crate::Result<Option<Request<Option<T>>>> {
        let mut req = httparse::Request::new(&mut []);
        let mut headers: [MaybeUninit<httparse::Header>; 64] =
            unsafe { MaybeUninit::uninit().assume_init() };

        match req.parse_with_uninit_headers(bytes.as_ref(), &mut headers)? {
            Status::Complete(_body_offset) => {
                //todo!("body_offset we need to save to parse body")
                parse_request_header(req).map(|request| Some(request))
            }
            Status::Partial => {
                Ok(None)
            }
        }
    }
}