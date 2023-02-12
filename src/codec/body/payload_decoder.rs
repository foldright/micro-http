use crate::codec::{ChunkedDecoder, LengthDecoder};
use bytes::{Bytes, BytesMut};
use std::io;
use tokio_util::codec::Decoder;

/// payload item produced from payload decoder
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayloadItem {
    Chunk(Bytes),
    Eof,
}

/// decode payload for request body
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadDecoder {
    kind: Kind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    /// content-length payload
    Length(LengthDecoder),

    /// transfer-encoding chunked payload
    Chunked(ChunkedDecoder),
}

impl PayloadItem {
    pub fn is_eof(&self) -> bool {
        match self {
            PayloadItem::Chunk(_) => false,
            PayloadItem::Eof => true,
        }
    }

    pub fn is_chunk(&self) -> bool {
        match self {
            PayloadItem::Chunk(_) => true,
            PayloadItem::Eof => false,
        }
    }

    pub fn bytes(&self) -> Option<&Bytes> {
        match self {
            PayloadItem::Chunk(bytes) => Some(bytes),
            PayloadItem::Eof => None,
        }
    }

    pub fn bytes_mut(&mut self) -> Option<&mut Bytes> {
        match self {
            PayloadItem::Chunk(bytes) => Some(bytes),
            PayloadItem::Eof => None,
        }
    }
}

impl PayloadDecoder {
    /// create an empty `PayloadDecoder`
    pub fn empty() -> Self {
        Self { kind: Kind::Length(LengthDecoder::new(0)) }
    }

    /// create a chunked `PayloadDecoder`
    pub fn chunked() -> Self {
        Self { kind: Kind::Chunked(ChunkedDecoder::new()) }
    }

    /// create a fixed length `PayloadDecoder`
    pub fn length(size: usize) -> Self {
        Self { kind: Kind::Length(LengthDecoder::new(size)) }
    }
}

impl Decoder for PayloadDecoder {
    type Item = PayloadItem;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match &mut self.kind {
            Kind::Length(length_decoder) => length_decoder.decode(src),
            Kind::Chunked(chunked_decoder) => chunked_decoder.decode(src),
        }
    }
}
