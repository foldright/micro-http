use std::mem::MaybeUninit;
use bytes::{Buf, BytesMut};


use httparse::Status;
use tokio_util::codec::{Decoder};
use crate::protocol::RequestHeader;

pub struct HeaderDecoder;

impl Decoder for HeaderDecoder {
    type Item = RequestHeader;
    type Error = crate::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let mut req = httparse::Request::new(&mut []);
        let mut headers: [MaybeUninit<httparse::Header>; 64] =
            unsafe { MaybeUninit::uninit().assume_init() };

        match req.parse_with_uninit_headers(src.as_ref(), &mut headers)? {
            Status::Complete(body_offset) => {
                let header = req.into();
                src.advance(body_offset);
                Ok(Some(header))
            }
            Status::Partial => Ok(None),
        }
    }
}


#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;

    #[test]
    fn test_bytes_mut_lens() {
        let str = indoc! {r##"
        GET /index.html HTTP/1.1
        Host: 127.0.0.1:8080
        User-Agent: curl/7.79.1
        Accept: */*

        123"##};

        let mut bytes = BytesMut::from(str);

        assert_eq!(bytes.len(), str.len());

        let mut header_decoder = HeaderDecoder;

        let result = header_decoder.decode(&mut bytes).unwrap();

        assert!(result.is_some());

        assert_eq!(bytes.len(), 3);
        assert_eq!(&bytes[..], &b"123"[..]);
    }
}


