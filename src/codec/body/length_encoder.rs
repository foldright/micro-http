

use crate::protocol::PayloadItem;
use bytes::BytesMut;
use tokio_util::codec::Encoder;
use tracing::warn;
use crate::codec::EncoderError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LengthEncoder {
    length: u64,
}

impl LengthEncoder {
    pub fn new(length: u64) -> Self {
        Self { length }
    }
}

impl Encoder<PayloadItem> for LengthEncoder {
    type Error = EncoderError;

    fn encode(&mut self, item: PayloadItem, dst: &mut BytesMut) -> Result<(), Self::Error> {
        if self.length == 0 {
            warn!("encode payload_item but no need to encode anymore");
            return Ok(());
        }

        match item {
            PayloadItem::Chunk(bytes) => {
                if bytes.len() == 0 {
                    return Ok(());
                }
                dst.extend_from_slice(&bytes[..]);
                self.length -= bytes.len() as u64;
                Ok(())
            }
            PayloadItem::Eof => Ok(()),
        }
    }
}
