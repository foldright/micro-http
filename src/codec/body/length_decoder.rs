use bytes::BytesMut;
use tokio_util::codec::Decoder;
use crate::codec::body::BodyData;

pub struct LengthDecoder {
    length: usize,
}

impl LengthDecoder {
    pub fn new(length: usize) -> Self {
        Self {
            length,
        }
    }
}


impl Decoder for LengthDecoder {
    type Item = BodyData;
    type Error = crate::Error;

    // TODO : the lengthDecoder will buffer all the content in the memory
    //       we can set up a size point, and let the large bytes save in the disk
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if self.length == 0 {
            return Ok(Some(BodyData::Finished));
        }

        if src.len() < self.length {
            return Ok(None);
        }

        let bytes = src.split_to(self.length).freeze();
        self.length -= bytes.len();

        Ok(Some(BodyData::Bytes(bytes)))
    }
}