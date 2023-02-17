use std::cmp;

use crate::protocol::{ParseError, PayloadItem};
use bytes::BytesMut;
use tokio_util::codec::Decoder;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LengthDecoder {
    length: u64,
}

impl LengthDecoder {
    pub fn new(length: u64) -> Self {
        Self { length }
    }
}

impl Decoder for LengthDecoder {
    type Item = PayloadItem;
    type Error = ParseError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if self.length == 0 {
            return Ok(Some(PayloadItem::Eof));
        }

        if src.len() == 0 {
            return Ok(None);
        }

        let len = cmp::min(self.length, src.len() as u64);
        let bytes = src.split_to(len as usize).freeze();

        self.length -= bytes.len() as u64;
        Ok(Some(PayloadItem::Chunk(bytes)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut buffer: BytesMut = BytesMut::from(&b"101234567890abcdef\r\n\r\n"[..]);

        let mut length_decoder = LengthDecoder::new(10);
        let item = length_decoder.decode(&mut buffer);

        let payload = item.unwrap().unwrap();
        assert!(payload.is_chunk());

        let bytes = payload.as_bytes().unwrap();

        assert_eq!(bytes.len(), 10);

        assert_eq!(&bytes[..], b"1012345678");
        assert_eq!(&buffer[..], b"90abcdef\r\n\r\n");
    }
}
