use crate::protocol::{PayloadItem, SendError};
use bytes::{Buf, BytesMut};
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

impl <D: Buf> Encoder<PayloadItem<D>> for ChunkedEncoder {
    type Error = SendError;

    fn encode(&mut self, item: PayloadItem<D>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        if self.eof {
            return Ok(());
        }

        match item {
            PayloadItem::Chunk(bytes) => {
                write!(helper::Writer(dst), "{:X\r\n}", bytes.remaining())?;
                dst.reserve(bytes.remaining() + 2);
                dst.extend_from_slice(bytes.chunk());
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
