use crate::codec::body::PayloadDecoder;
use crate::codec::header::HeaderDecoder;
use crate::protocol::{Message, ParseError, PayloadItem, RequestHeader};
use bytes::BytesMut;
use tokio_util::codec::Decoder;

pub struct RequestDecoder {
    header_decoder: HeaderDecoder,
    payload_decoder: Option<PayloadDecoder>,
}

impl RequestDecoder {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for RequestDecoder {
    fn default() -> Self {
        Self { header_decoder: HeaderDecoder, payload_decoder: None }
    }
}

impl Decoder for RequestDecoder {
    type Item = Message<RequestHeader>;
    type Error = ParseError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // parse payload if have payload_decoder
        if let Some(payload_decoder) = &mut self.payload_decoder {
            let message = match payload_decoder.decode(src)? {
                Some(item @ PayloadItem::Chunk(_)) => Some(Message::Payload(item)),
                Some(item @ PayloadItem::Eof) => {
                    // no need payload decoder in this request now
                    self.payload_decoder.take();
                    Some(Message::Payload(item))
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
