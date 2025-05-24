use bytes::{BufMut, BytesMut};
use futures::{AsyncWrite, TryFutureExt};
use tokio::io::{AsyncRead, AsyncReadExt};
use crate::protocol::{ParseError, RequestHeader};

pub struct HttpConnection2<R, W> {
    reader: R,
    writer: W,
    reader_buf: BytesMut,
    writer_buf: BytesMut,
}


impl<R, W> HttpConnection2<R, W> {
    pub fn with_capacity(reader: R, writer: W, reader_buffer_size: usize, writer_buffer_size: usize) -> HttpConnection2<R, W> {
        Self {
            reader,
            writer,
            reader_buf: BytesMut::with_capacity(reader_buffer_size),
            writer_buf: BytesMut::with_capacity(writer_buffer_size),
        }
    }
}

impl<R: AsyncRead + Unpin, W: AsyncWrite> HttpConnection2<R, W> {
    pub async fn read_request_header(&mut self) -> Result<RequestHeader, ParseError> {

        match self.reader.read_buf(&mut self.reader_buf).await.map_err(ParseError::io)? {
            _ => {}
        }
        


        todo!()
    }
}