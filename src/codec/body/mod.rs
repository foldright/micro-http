use bytes::{Bytes, BytesMut};
use tokio_util::codec::Decoder;
use crate::codec::{ChunkedDecoder, LengthDecoder};
use crate::protocol::body::BodyLength;

pub mod length_decoder;
pub mod chunked_decoder;

pub enum BodyDecoder {
    Fixed(LengthDecoder),
    Chunked(ChunkedDecoder),
}

pub enum BodyData {
    Bytes(Bytes),
    Finished,
}

impl From<BodyLength> for BodyDecoder {
    fn from(body_length: BodyLength) -> Self {
        match body_length {
            BodyLength::Fix(n) => BodyDecoder::Fixed(LengthDecoder::new(n)),
            BodyLength::Chunked => BodyDecoder::Chunked(ChunkedDecoder::new()),
        }
    }
}

impl Decoder for BodyDecoder {
    type Item = BodyData;
    type Error = crate::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self {
            BodyDecoder::Fixed(length_decoder) => length_decoder.decode(src),
            BodyDecoder::Chunked(chunked_decoder) => chunked_decoder.decode(src),
        }
    }
}

impl BodyData {
    fn into_bytes(self) -> Option<Bytes> {
        match self {
            Self::Bytes(bytes) => Some(bytes),
            Self::Finished => None,
        }
    }

    fn is_finished(&self) -> bool {
        match self {
            Self::Bytes(_) => false,
            Self::Finished => true,
        }
    }
}