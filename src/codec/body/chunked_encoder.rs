use crate::protocol::PayloadItem;
use bytes::BytesMut;
use std::io::Write;

use tokio_util::codec::Encoder;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkedEncoder {
    eof: bool,
    send_size: usize,
}

impl ChunkedEncoder {
    pub fn new() -> Self {
        Self { eof: false, send_size: 0 }
    }
}

impl Encoder<PayloadItem> for ChunkedEncoder {
    type Error = std::io::Error;

    fn encode(&mut self, item: PayloadItem, dst: &mut BytesMut) -> Result<(), Self::Error> {
        if self.eof {
            return Ok(());
        }

        match item {
            PayloadItem::Chunk(bytes) => {
                write!(helper::Writer(dst), "{:X\r\n}", bytes.len())?;
                dst.reserve(bytes.len() + 2);
                dst.extend_from_slice(&bytes[..]);
                dst.extend_from_slice(b"\r\n");
                Ok(())
            }
            PayloadItem::Eof => {
                self.eof = true;
                dst.extend_from_slice(b"0\r\n\r\n");
                Ok(())
            }
        }
    }
}

mod helper {
    use bytes::{BufMut, BytesMut};
    use std::io;

    pub struct Writer<'a>(pub &'a mut BytesMut);

    impl<'a> io::Write for Writer<'a> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.put_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
}
