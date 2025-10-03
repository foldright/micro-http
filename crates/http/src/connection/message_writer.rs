use crate::codec::ResponseEncoder;
use crate::protocol::{Message, PayloadSize, ResponseHead, SendError};
use bytes::{Buf, BytesMut};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio_util::codec::Encoder;

#[derive(Debug)]
pub struct MessageWriter<W> {
    writer: W,
    buffer: BytesMut,
    encoder: ResponseEncoder,
}

impl<W> MessageWriter<W>
where
    W: AsyncWrite + Unpin,
{
    pub fn with_capacity(writer: W, buffer_size: usize) -> Self {
        Self { writer, buffer: BytesMut::with_capacity(buffer_size), encoder: ResponseEncoder::new() }
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    pub fn clear_buf(&mut self) {
        self.buffer.clear();
    }

    #[inline]
    pub fn write<D>(&mut self, item: Message<(ResponseHead, PayloadSize), D>) -> Result<(), SendError>
    where
        D: Buf,
    {
        self.encoder.encode(item, &mut self.buffer)
    }

    #[inline]
    pub async fn flush(&mut self) -> Result<(), SendError> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        self.writer.write_all(self.buffer.as_ref()).await?;
        Ok(self.writer.flush().await?)
    }
}
