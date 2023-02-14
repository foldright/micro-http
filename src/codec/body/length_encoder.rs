

use crate::protocol::PayloadItem;
use bytes::BytesMut;
use tokio_util::codec::Encoder;
use crate::codec::EncoderError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LengthEncoder {
    length: usize,
}

impl LengthEncoder {
    pub fn new(length: usize) -> Self {
        Self { length }
    }
}

impl Encoder<PayloadItem> for LengthEncoder {
    type Error = EncoderError;

    fn encode(&mut self, item: PayloadItem, dst: &mut BytesMut) -> Result<(), Self::Error> {
        if self.length == 0 {
            return Ok(());
        }

        match item {
            PayloadItem::Chunk(bytes) => {
                if bytes.len() == 0 {
                    return Ok(());
                }
                dst.extend_from_slice(&bytes[..]);
                Ok(())
            }
            PayloadItem::Eof => Ok(()),
        }
    }
}
