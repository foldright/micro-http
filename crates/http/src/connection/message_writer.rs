use crate::codec::{HeaderEncoder, ResponseEncoder};
use crate::protocol::{Message, PayloadSize, ResponseHead, SendError};
use bytes::{Buf, BytesMut};
use std::io::{self, IoSlice, Write};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio_util::codec::Encoder;

#[derive(Debug)]
pub struct MessageWriter<W> {
    writer: W,
    buffer: BytesMut,
    remaining_length: Option<u64>,
    #[allow(dead_code)]
    encoder: ResponseEncoder,
}

impl<W> MessageWriter<W>
where
    W: AsyncWrite + Unpin,
{
    pub fn with_capacity(writer: W, buffer_size: usize) -> Self {
        Self { writer, buffer: BytesMut::with_capacity(buffer_size), remaining_length: None, encoder: ResponseEncoder::new() }
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    #[allow(dead_code)]
    pub fn clear_buf(&mut self) {
        self.buffer.clear();
    }

    #[allow(dead_code)]
    #[inline]
    pub fn write<D>(&mut self, item: Message<(ResponseHead, PayloadSize), D>) -> Result<(), SendError>
    where
        D: Buf,
    {
        self.encoder.encode(item, &mut self.buffer)
    }

    #[allow(dead_code)]
    #[inline]
    pub async fn flush(&mut self) -> Result<(), SendError> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        self.writer.write_all(self.buffer.as_ref()).await?;
        Ok(self.writer.flush().await?)
    }

    #[inline]
    pub async fn send(&mut self) -> Result<(), SendError> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        self.writer.write_all(self.buffer.as_ref()).await?;
        Ok(())
    }

    pub async fn send_head(&mut self, head: ResponseHead, payload_size: PayloadSize) -> Result<(), SendError> {
        let mut encoder = HeaderEncoder;
        encoder.encode((head, payload_size), &mut self.buffer)?;
        self.send().await?;
        self.buffer.clear();
        Ok(())
    }

    pub async fn send_head_and_body(&mut self, head: ResponseHead, payload_size: PayloadSize, body: &[u8]) -> Result<(), SendError> {
        let PayloadSize::Length(length) = payload_size else {
            return Err(SendError::invalid_body("send_head_and_body requires a content-length response"));
        };
        let remaining = checked_remaining_length(length, body.len())?;

        let mut encoder = HeaderEncoder;
        encoder.encode((head, payload_size), &mut self.buffer)?;
        self.send_buffer_and_bytes(body).await?;
        self.remaining_length = Some(remaining);
        Ok(())
    }

    pub async fn send_body(&mut self, body: &[u8]) -> Result<(), SendError> {
        if body.is_empty() {
            return Ok(());
        }

        let remaining =
            self.remaining_length.ok_or_else(|| SendError::invalid_body("received a fixed-length body chunk without a response head"))?;
        let next_remaining = checked_remaining_length(remaining, body.len())?;
        self.writer.write_all(body).await?;
        self.remaining_length = Some(next_remaining);
        Ok(())
    }

    pub fn finish_body(&mut self) -> Result<(), SendError> {
        match self.remaining_length.take() {
            Some(0) => Ok(()),
            Some(remaining) => Err(SendError::invalid_body(format!("response body ended with {remaining} content-length bytes remaining"))),
            None => Err(SendError::invalid_body("finished a fixed-length body before sending its response head")),
        }
    }

    pub async fn send_head_and_chunk(&mut self, head: ResponseHead, body: &[u8]) -> Result<(), SendError> {
        let payload_size = PayloadSize::Chunked;
        let mut encoder = HeaderEncoder;
        encoder.encode((head, payload_size), &mut self.buffer)?;
        if body.is_empty() {
            self.send().await?;
            self.buffer.clear();
            return Ok(());
        }

        self.encode_chunk_prefix(body.len())?;
        self.send_buffer_and_bytes(body).await?;
        self.writer.write_all(b"\r\n").await?;
        Ok(())
    }

    pub async fn send_chunk(&mut self, body: &[u8]) -> Result<(), SendError> {
        if body.is_empty() {
            return Ok(());
        }

        self.encode_chunk_prefix(body.len())?;
        self.send_buffer_and_bytes(body).await?;
        self.writer.write_all(b"\r\n").await?;
        Ok(())
    }

    pub async fn send_chunk_eof(&mut self) -> Result<(), SendError> {
        self.writer.write_all(b"0\r\n\r\n").await?;
        Ok(())
    }

    fn encode_chunk_prefix(&mut self, len: usize) -> Result<(), SendError> {
        write!(ChunkPrefixWriter(&mut self.buffer), "{len:X}\r\n")?;
        Ok(())
    }

    async fn send_buffer_and_bytes(&mut self, body: &[u8]) -> Result<(), SendError> {
        let mut header = self.buffer.as_ref();
        let mut payload = body;

        while !header.is_empty() || !payload.is_empty() {
            if header.is_empty() {
                self.writer.write_all(payload).await?;
                break;
            }

            if payload.is_empty() {
                self.writer.write_all(header).await?;
                break;
            }

            if self.writer.is_write_vectored() {
                let bufs = [IoSlice::new(header), IoSlice::new(payload)];
                let written = self.writer.write_vectored(&bufs).await?;
                if written == 0 {
                    return Err(SendError::io(io::Error::from(io::ErrorKind::WriteZero)));
                }

                if written < header.len() {
                    header = &header[written..];
                    continue;
                }

                let body_written = written - header.len();
                header = &[];
                payload = &payload[body_written..];
                continue;
            }

            self.writer.write_all(header).await?;
            header = &[];
        }

        self.buffer.clear();
        Ok(())
    }
}

fn checked_remaining_length(remaining: u64, chunk_len: usize) -> Result<u64, SendError> {
    let chunk_len = u64::try_from(chunk_len).map_err(|_| SendError::invalid_body("response body chunk length exceeds u64"))?;
    remaining
        .checked_sub(chunk_len)
        .ok_or_else(|| SendError::invalid_body(format!("response body exceeds content-length by {} bytes", chunk_len - remaining)))
}

struct ChunkPrefixWriter<'a>(&'a mut BytesMut);

impl Write for ChunkPrefixWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::PayloadItem;
    use bytes::Bytes;
    use http::Response;
    use std::pin::Pin;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::task::{Context, Poll};

    #[derive(Debug, Default)]
    struct CountingWriter {
        bytes: Vec<u8>,
        writes: Arc<AtomicUsize>,
        vectored_writes: Arc<AtomicUsize>,
        flushes: Arc<AtomicUsize>,
    }

    impl CountingWriter {
        fn as_bytes(&self) -> &[u8] {
            &self.bytes
        }

        fn write_count(&self) -> usize {
            self.writes.load(Ordering::Relaxed)
        }

        fn vectored_write_count(&self) -> usize {
            self.vectored_writes.load(Ordering::Relaxed)
        }

        fn flush_count(&self) -> usize {
            self.flushes.load(Ordering::Relaxed)
        }
    }

    impl AsyncWrite for CountingWriter {
        fn poll_write(mut self: Pin<&mut Self>, _: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
            self.writes.fetch_add(1, Ordering::Relaxed);
            self.bytes.extend_from_slice(buf);
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_write_vectored(mut self: Pin<&mut Self>, _: &mut Context<'_>, bufs: &[IoSlice<'_>]) -> Poll<io::Result<usize>> {
            self.vectored_writes.fetch_add(1, Ordering::Relaxed);
            let mut written = 0;
            for buf in bufs {
                self.bytes.extend_from_slice(buf.as_ref());
                written += buf.len();
            }
            Poll::Ready(Ok(written))
        }

        fn is_write_vectored(&self) -> bool {
            true
        }

        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
            self.flushes.fetch_add(1, Ordering::Relaxed);
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    #[tokio::test]
    async fn send_does_not_flush_underlying_writer() {
        let writer = CountingWriter::default();
        let mut message_writer = MessageWriter::with_capacity(writer, 256);
        let response = Response::builder().status(200).body(()).unwrap();
        let head = response;

        message_writer.write(Message::<_, Bytes>::Header((head, PayloadSize::Empty))).unwrap();
        message_writer.write(Message::Payload(PayloadItem::<Bytes>::Eof)).unwrap();
        message_writer.send().await.unwrap();

        let writer = message_writer.get_mut();
        assert_eq!(writer.write_count(), 1);
        assert_eq!(writer.flush_count(), 0);
    }

    #[tokio::test]
    async fn flush_still_flushes_underlying_writer() {
        let writer = CountingWriter::default();
        let mut message_writer = MessageWriter::with_capacity(writer, 256);
        let response = Response::builder().status(200).body(()).unwrap();
        let head = response;

        message_writer.write(Message::<_, Bytes>::Header((head, PayloadSize::Empty))).unwrap();
        message_writer.write(Message::Payload(PayloadItem::<Bytes>::Eof)).unwrap();
        message_writer.flush().await.unwrap();

        let writer = message_writer.get_mut();
        assert_eq!(writer.write_count(), 1);
        assert_eq!(writer.flush_count(), 1);
    }

    #[tokio::test]
    async fn send_head_and_body_uses_vectored_write() {
        let writer = CountingWriter::default();
        let mut message_writer = MessageWriter::with_capacity(writer, 256);
        let response = Response::builder().status(200).body(()).unwrap();
        let body = Bytes::from_static(b"hello");

        message_writer.send_head_and_body(response, PayloadSize::Length(body.len() as u64), body.as_ref()).await.unwrap();

        let writer = message_writer.get_mut();
        let text = std::str::from_utf8(writer.as_bytes()).unwrap();
        assert!(text.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(text.contains("content-length: 5\r\n"));
        assert!(text.ends_with("\r\nhello"));
        assert_eq!(writer.vectored_write_count(), 1);
        assert_eq!(writer.flush_count(), 0);
    }

    #[tokio::test]
    async fn fixed_length_body_must_end_at_declared_length() {
        let writer = CountingWriter::default();
        let mut message_writer = MessageWriter::with_capacity(writer, 256);
        let response = Response::builder().status(200).body(()).unwrap();

        message_writer.send_head_and_body(response, PayloadSize::Length(6), b"hello").await.unwrap();

        let error = message_writer.finish_body().unwrap_err();
        assert!(error.to_string().contains("1 content-length bytes remaining"));
    }

    #[tokio::test]
    async fn fixed_length_body_rejects_excess_before_writing_it() {
        let writer = CountingWriter::default();
        let mut message_writer = MessageWriter::with_capacity(writer, 256);
        let response = Response::builder().status(200).body(()).unwrap();

        message_writer.send_head_and_body(response, PayloadSize::Length(6), b"hello").await.unwrap();
        let error = message_writer.send_body(b"!!").await.unwrap_err();

        assert!(error.to_string().contains("exceeds content-length by 1 bytes"));
        assert!(message_writer.get_mut().as_bytes().ends_with(b"hello"));
    }

    #[tokio::test]
    async fn fixed_length_body_rejects_oversized_first_chunk_before_writing_head() {
        let writer = CountingWriter::default();
        let mut message_writer = MessageWriter::with_capacity(writer, 256);
        let response = Response::builder().status(200).body(()).unwrap();

        let error = message_writer.send_head_and_body(response, PayloadSize::Length(4), b"hello").await.unwrap_err();

        assert!(error.to_string().contains("exceeds content-length by 1 bytes"));
        assert!(message_writer.get_mut().as_bytes().is_empty());
    }

    #[tokio::test]
    async fn fixed_length_body_accepts_multiple_exact_chunks() {
        let writer = CountingWriter::default();
        let mut message_writer = MessageWriter::with_capacity(writer, 256);
        let response = Response::builder().status(200).body(()).unwrap();

        message_writer.send_head_and_body(response, PayloadSize::Length(11), b"hello").await.unwrap();
        message_writer.send_body(b" world").await.unwrap();
        message_writer.finish_body().unwrap();

        assert!(message_writer.get_mut().as_bytes().ends_with(b"hello world"));
    }

    #[tokio::test]
    async fn send_head_and_chunk_writes_chunked_framing() {
        let writer = CountingWriter::default();
        let mut message_writer = MessageWriter::with_capacity(writer, 256);
        let response = Response::builder().status(200).body(()).unwrap();
        let body = Bytes::from_static(b"hello");

        message_writer.send_head_and_chunk(response, body.as_ref()).await.unwrap();
        message_writer.send_chunk_eof().await.unwrap();

        let writer = message_writer.get_mut();
        let text = std::str::from_utf8(writer.as_bytes()).unwrap();
        assert!(text.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(text.contains("transfer-encoding: chunked\r\n"));
        assert!(text.ends_with("5\r\nhello\r\n0\r\n\r\n"));
    }

    #[tokio::test]
    async fn empty_first_chunk_does_not_terminate_chunked_body() {
        let writer = CountingWriter::default();
        let mut message_writer = MessageWriter::with_capacity(writer, 256);
        let response = Response::builder().status(200).body(()).unwrap();

        message_writer.send_head_and_chunk(response, b"").await.unwrap();
        message_writer.send_chunk(b"hello").await.unwrap();
        message_writer.send_chunk_eof().await.unwrap();

        let text = std::str::from_utf8(message_writer.get_mut().as_bytes()).unwrap();
        assert!(text.ends_with("\r\n5\r\nhello\r\n0\r\n\r\n"));
        assert!(!text.ends_with("\r\n0\r\n\r\n5\r\nhello\r\n0\r\n\r\n"));
    }
}
