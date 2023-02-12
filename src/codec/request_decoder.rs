use crate::codec::body::payload_decoder::PayloadDecoder;
use crate::codec::header_decoder::HeaderDecoder;
use crate::codec::{ParseError, PayloadItem};
use crate::protocol::RequestHeader;
use bytes::{Bytes, BytesMut};
use tokio_util::codec::Decoder;

pub enum Message {
    Header(RequestHeader),
    Chunked(Option<Bytes>),
}

pub struct RequestDecoder {
    header_decoder: HeaderDecoder,
    payload_decoder: Option<PayloadDecoder>,
}

impl RequestDecoder {
    pub fn new() -> Self {
        Self { header_decoder: HeaderDecoder, payload_decoder: None }
    }
}

impl Decoder for RequestDecoder {
    type Item = Message;
    type Error = ParseError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // parse payload if have payload_decoder
        if let Some(payload_decoder) = &mut self.payload_decoder {
            let message = match payload_decoder.decode(src)? {
                Some(PayloadItem::Chunk(bytes)) => Some(Message::Chunked(Some(bytes))),
                Some(PayloadItem::Eof) => {
                    // no need payload decoder in this request now
                    self.payload_decoder.take();
                    Some(Message::Chunked(None))
                }
                None => None,
            };

            return Ok(message);
        }

        // parse request
        let message = match self.header_decoder.decode(src)? {
            Some((header, payload_decoder)) => {
                self.payload_decoder = Some(payload_decoder);
                Some(Message::Header(header))
            }
            None => None,
        };

        Ok(message)
    }
}
