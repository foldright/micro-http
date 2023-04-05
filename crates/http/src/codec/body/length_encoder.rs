use crate::protocol::{PayloadItem, SendError};
use bytes::{Buf, BytesMut};
use tokio_util::codec::Encoder;
use tracing::warn;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LengthEncoder {
    received_eof: bool,
    length: u64,
}

impl LengthEncoder {
    pub fn new(length: u64) -> Self {
        Self { received_eof: false, length }
    }

    pub fn is_finish(&self) -> bool {
        self.length == 0 && self.received_eof
    }
}

impl<D: Buf> Encoder<PayloadItem<D>> for LengthEncoder {
    type Error = SendError;

    fn encode(&mut self, item: PayloadItem<D>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        if self.length == 0 && !item.is_eof() {
            warn!("encode payload_item but no need to encode anymore");
            return Ok(());
        }

        match item {
            PayloadItem::Chunk(bytes) => {
                if !bytes.has_remaining() {
                    return Ok(());
                }
                dst.extend_from_slice(bytes.chunk());
                self.length -= bytes.remaining() as u64;
                Ok(())
            }
            PayloadItem::Eof => {
                self.received_eof = true;
                Ok(())
            }
        }
    }
}
