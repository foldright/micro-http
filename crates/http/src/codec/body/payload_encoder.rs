use crate::codec::body::chunked_encoder::ChunkedEncoder;
use crate::codec::body::length_encoder::LengthEncoder;
use crate::protocol::{PayloadItem, SendError};
use bytes::{Buf, BytesMut};

use tokio_util::codec::Encoder;

/// encode payload for request body
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadEncoder {
    kind: Kind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    /// content-length payload
    Length(LengthEncoder),

    /// transfer-encoding chunked payload
    Chunked(ChunkedEncoder),

    /// have no body with the request
    NoBody,
}

impl PayloadEncoder {
    /// create an empty `PayloadEncoder`
    /// #[allow(unused)]
    pub fn empty() -> Self {
        Self { kind: Kind::NoBody }
    }

    /// create a chunked `PayloadEncoder`
    /// #[allow(unused)]
    pub fn chunked() -> Self {
        Self { kind: Kind::Chunked(ChunkedEncoder::new()) }
    }

    /// create a fixed length `PayloadEncoder`
    #[allow(unused)]
    pub fn fix_length(size: u64) -> Self {
        Self { kind: Kind::Length(LengthEncoder::new(size)) }
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

    pub fn is_finish(&self) -> bool {
        match &self.kind {
            Kind::Length(encoder) => encoder.is_finish(),
            Kind::Chunked(encoder) => encoder.is_finish(),
            Kind::NoBody => true,
        }
    }
}

impl<D: Buf> Encoder<PayloadItem<D>> for PayloadEncoder {
    type Error = SendError;

    fn encode(&mut self, item: PayloadItem<D>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match &mut self.kind {
            Kind::Length(encoder) => encoder.encode(item, dst),
            Kind::Chunked(encoder) => encoder.encode(item, dst),
            Kind::NoBody => Ok(()),
        }
    }
}
