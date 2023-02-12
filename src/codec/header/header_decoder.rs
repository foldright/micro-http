use std::mem::MaybeUninit;

use crate::codec::body::PayloadDecoder;
use crate::codec::error::TooLargeHeaderSnafu;
use bytes::{Buf, BytesMut};
use http::HeaderValue;
use httparse::{Error, Status};
use snafu::ensure;
use tokio_util::codec::Decoder;
use tracing::trace;

use crate::codec::ParseError;
use crate::codec::ParseError::{InvalidContentLength, InvalidHeader, TooManyHeaders};
use crate::protocol::RequestHeader;

const MAX_HEADER_NUM: usize = 64;
const MAX_HEADER_BYTES: usize = 8 * 1024;

pub struct HeaderDecoder;

impl Decoder for HeaderDecoder {
    type Item = (RequestHeader, PayloadDecoder);
    type Error = ParseError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let mut req = httparse::Request::new(&mut []);
        let mut headers: [MaybeUninit<httparse::Header>; MAX_HEADER_NUM] =
            unsafe { MaybeUninit::uninit().assume_init() };

        let parsed_result = req.parse_with_uninit_headers(src.as_ref(), &mut headers).map_err(|e| match e {
            Error::TooManyHeaders => TooManyHeaders { max_num: MAX_HEADER_NUM, source: e },
            _ => InvalidHeader { source: e },
        });

        match parsed_result? {
            Status::Complete(body_offset) => {
                trace!(body_size = body_offset, "parsed body size");
                ensure!(
                    body_offset <= MAX_HEADER_BYTES,
                    TooLargeHeaderSnafu { current_size: body_offset, max_size: MAX_HEADER_BYTES }
                );

                let header = req.into();
                let payload_decoder = parse_payload(&header)?;

                src.advance(body_offset);
                Ok(Some((header, payload_decoder)))
            }
            Status::Partial => {
                ensure!(
                    src.len() <= MAX_HEADER_BYTES,
                    TooLargeHeaderSnafu { current_size: src.len(), max_size: MAX_HEADER_BYTES }
                );
                Ok(None)
            }
        }
    }
}

fn parse_payload(header: &RequestHeader) -> Result<PayloadDecoder, ParseError> {
    if !header.need_body() {
        return Ok(PayloadDecoder::empty());
    }

    // refer: https://www.rfc-editor.org/rfc/rfc7230#section-3.3
    let te_header = header.headers().get(http::header::TRANSFER_ENCODING);
    let cl_header = header.headers().get(http::header::CONTENT_LENGTH);

    match (te_header, cl_header) {
        (None, None) => Ok(PayloadDecoder::empty()),
        (te_value @ Some(_), None) => {
            if is_chunked(te_value) {
                Ok(PayloadDecoder::chunked())
            } else {
                Ok(PayloadDecoder::empty())
            }
        }
        (None, Some(cl_value)) => {
            let cl_str = cl_value.to_str().map_err(|_| InvalidContentLength { message: "can't to_str".into() })?;

            let length = cl_str.trim().parse::<usize>().map_err(|_| InvalidContentLength { message: cl_str.into() })?;

            Ok(PayloadDecoder::length(length))
        }

        (Some(_), Some(_)) => {
            Err(InvalidContentLength { message: "transfer_encoding and content_length both present in headers".into() })
        }
    }
}

fn is_chunked(header_value: Option<&HeaderValue>) -> bool {
    header_value
        .and_then(|value| value.to_str().ok())
        .and_then(|encodings| encodings.rsplit(',').next())
        .map(|last_encoding| last_encoding.trim() == "chunked")
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderMap;
    use indoc::indoc;

    #[test]
    fn check_is_chunked() {
        {
            let headers = HeaderMap::new();
            assert!(!is_chunked(headers.get(http::header::TRANSFER_ENCODING)))
        }

        {
            let mut headers = HeaderMap::new();
            headers.insert("Accept", "foo".parse().unwrap());
            headers.insert("Transfer-Encoding", "gzip, chunked".parse().unwrap());
            headers.insert("Host", "bar".parse().unwrap());
            assert!(is_chunked(headers.get(http::header::TRANSFER_ENCODING)));
        }

        {
            let mut headers = HeaderMap::new();
            headers.insert("Accept", "foo".parse().unwrap());
            headers.insert("Transfer-Encoding", "chunked, gzip".parse().unwrap());
            headers.insert("Host", "bar".parse().unwrap());
            assert!(!is_chunked(headers.get(http::header::TRANSFER_ENCODING)));
        }

        {
            let mut headers = HeaderMap::new();
            headers.insert("Accept", "foo".parse().unwrap());
            headers.insert("Transfer-Encoding", "gzip".parse().unwrap());
            headers.insert("Host", "bar".parse().unwrap());
            assert!(!is_chunked(headers.get(http::header::TRANSFER_ENCODING)));
        }
    }

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
