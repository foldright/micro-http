use crate::codec::body::chunked_decoder::ChunkedDecoder;
use crate::codec::body::length_decoder::LengthDecoder;
use crate::protocol::PayloadItem;
use bytes::BytesMut;
use std::io;
use tokio_util::codec::Decoder;

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

    /// have no body with the request
    NoBody,
}

impl PayloadDecoder {
    /// create an empty `PayloadDecoder`
    #[allow(unused)]
    pub fn empty() -> Self {
        Self { kind: Kind::NoBody }
    }

    /// create a chunked `PayloadDecoder`
    pub fn chunked() -> Self {
        Self { kind: Kind::Chunked(ChunkedDecoder::new()) }
    }

    /// create a fixed length `PayloadDecoder`
    #[allow(unused)]
    pub fn fix_length(size: u64) -> Self {
        Self { kind: Kind::Length(LengthDecoder::new(size)) }
    }

    #[allow(unused)]
    pub fn is_chunked(&self) -> bool {
        match &self.kind {
            Kind::Length(_) => false,
            Kind::Chunked(_) => true,
            Kind::NoBody => false,
        }
    }

    #[allow(unused)]
    pub fn is_empty(&self) -> bool {
        match &self.kind {
            Kind::Length(_) => false,
            Kind::Chunked(_) => false,
            Kind::NoBody => true,
        }
    }

    #[allow(unused)]
    pub fn is_fix_length(&self) -> bool {
        match &self.kind {
            Kind::Length(_) => true,
            Kind::Chunked(_) => false,
            Kind::NoBody => false,
        }
    }
}

impl Decoder for PayloadDecoder {
    type Item = PayloadItem;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match &mut self.kind {
            Kind::Length(length_decoder) => length_decoder.decode(src),
            Kind::Chunked(chunked_decoder) => chunked_decoder.decode(src),
            Kind::NoBody => Ok(Some(PayloadItem::Eof)),
        }
    }
}
