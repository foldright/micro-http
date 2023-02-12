use std::{cmp, io};

use bytes::BytesMut;
use tokio_util::codec::Decoder;

use crate::codec::body::payload_decoder::PayloadItem;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LengthDecoder {
    length: usize,
}

impl LengthDecoder {
    pub fn new(length: usize) -> Self {
        Self { length }
    }
}

impl Decoder for LengthDecoder {
    type Item = PayloadItem;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if self.length == 0 {
            return Ok(Some(PayloadItem::Eof));
        }

        if src.len() == 0 {
            return Ok(None);
        }

        let len = cmp::min(self.length, src.len());
        let bytes = src.split_to(len).freeze();

        self.length -= len;
        Ok(Some(PayloadItem::Chunk(bytes)))
    }
}
