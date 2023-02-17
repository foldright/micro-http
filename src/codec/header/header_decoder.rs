use std::mem::MaybeUninit;

use crate::codec::body::PayloadDecoder;
use crate::codec::error::TooLargeHeaderSnafu;
use bytes::{Buf, BytesMut};
use http::HeaderValue;
use httparse::{Error, Status};
use snafu::ensure;
use tokio_util::codec::Decoder;
use tracing::trace;

use crate::codec::DecodeError;
use crate::codec::DecodeError::{InvalidContentLength, InvalidHeader, TooManyHeaders};
use crate::protocol::RequestHeader;

const MAX_HEADER_NUM: usize = 64;
const MAX_HEADER_BYTES: usize = 8 * 1024;

pub struct HeaderDecoder;

impl Decoder for HeaderDecoder {
    type Item = (RequestHeader, PayloadDecoder);
    type Error = DecodeError;

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

fn parse_payload(header: &RequestHeader) -> Result<PayloadDecoder, DecodeError> {
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

            let length = cl_str.trim().parse::<u64>().map_err(|_| InvalidContentLength { message: cl_str.into() })?;

            Ok(PayloadDecoder::fix_length(length))
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
    use http::{HeaderMap, Method, Version};
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

    #[test]
    fn from_curl() {
        let str = indoc! {r##"
        GET /index.html HTTP/1.1
        Host: 127.0.0.1:8080
        User-Agent: curl/7.79.1
        Accept: */*

        "##};

        let mut buf = BytesMut::from(str);

        let (header, payload_decoder) = HeaderDecoder.decode(&mut buf).unwrap().unwrap();

        assert!(payload_decoder.is_empty());

        assert_eq!(header.method(), &Method::GET);
        assert_eq!(header.version(), Version::HTTP_11);
        assert_eq!(header.uri().host(), None);
        assert_eq!(header.uri().path(), "/index.html");
        assert_eq!(header.uri().scheme(), None);
        assert_eq!(header.uri().query(), None);

        assert_eq!(header.headers().len(), 3);

        assert_eq!(header.headers().get(http::header::ACCEPT), Some(&HeaderValue::from_str("*/*").unwrap()));

        assert_eq!(header.headers().get(http::header::HOST), Some(&HeaderValue::from_str("127.0.0.1:8080").unwrap()));

        assert_eq!(
            header.headers().get(http::header::USER_AGENT),
            Some(&HeaderValue::from_str("curl/7.79.1").unwrap())
        );
    }

    #[test]
    fn from_edge() {
        let str = indoc! {r##"
        GET /index/?a=1&b=2&a=3 HTTP/1.1
        Host: 127.0.0.1:8080
        Connection: keep-alive
        Cache-Control: max-age=0
        sec-ch-ua: "#Not_A Brand";v="99", "Microsoft Edge";v="109", "Chromium";v="109"
        sec-ch-ua-mobile: ?0
        sec-ch-ua-platform: "macOS"
        Upgrade-Insecure-Requests: 1
        User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36 Edg/109.0.1518.52
        Accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9
        Sec-Fetch-Site: none
        Sec-Fetch-Mode: navigate
        Sec-Fetch-User: ?1
        Sec-Fetch-Dest: document
        Accept-Encoding: gzip, deflate, br
        Accept-Language: zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7

        "##};

        let mut buf = BytesMut::from(str);

        let (header, payload_decoder) = HeaderDecoder.decode(&mut buf).unwrap().unwrap();

        assert!(payload_decoder.is_empty());

        assert_eq!(header.method(), &Method::GET);
        assert_eq!(header.version(), Version::HTTP_11);
        assert_eq!(header.uri().host(), None);
        assert_eq!(header.uri().path(), "/index/");
        assert_eq!(header.uri().scheme(), None);
        assert_eq!(header.uri().query(), Some("a=1&b=2&a=3"));

        assert_eq!(header.headers().len(), 15);

        assert_eq!(header.headers().get(http::header::CONNECTION), Some(&HeaderValue::from_str("keep-alive").unwrap()));

        assert_eq!(
            header.headers().get(http::header::CACHE_CONTROL),
            Some(&HeaderValue::from_str("max-age=0").unwrap())
        );

        assert_eq!(
            header.headers().get("sec-ch-ua"),
            Some(
                &HeaderValue::from_str(r##""#Not_A Brand";v="99", "Microsoft Edge";v="109", "Chromium";v="109""##)
                    .unwrap()
            )
        );

        assert_eq!(header.headers().get("sec-ch-ua-mobile"), Some(&HeaderValue::from_str("?0").unwrap()));

        assert_eq!(header.headers().get("sec-ch-ua-platform"), Some(&HeaderValue::from_str("\"macOS\"").unwrap()));

        assert_eq!(
            header.headers().get(http::header::UPGRADE_INSECURE_REQUESTS),
            Some(&HeaderValue::from_str("1").unwrap())
        );

        assert_eq!(header.headers().get(http::header::USER_AGENT),
                   Some(&HeaderValue::from_str("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36 Edg/109.0.1518.52").unwrap()));

        assert_eq!(header.headers().get("Sec-Fetch-Site"), Some(&HeaderValue::from_str("none").unwrap()));

        assert_eq!(header.headers().get("Sec-Fetch-Mode"), Some(&HeaderValue::from_str("navigate").unwrap()));

        assert_eq!(header.headers().get("Sec-Fetch-User"), Some(&HeaderValue::from_str("?1").unwrap()));

        assert_eq!(header.headers().get("Sec-Fetch-Dest"), Some(&HeaderValue::from_str("document").unwrap()));

        assert_eq!(
            header.headers().get(http::header::ACCEPT_ENCODING),
            Some(&HeaderValue::from_str("gzip, deflate, br").unwrap())
        );

        assert_eq!(
            header.headers().get(http::header::ACCEPT_LANGUAGE),
            Some(&HeaderValue::from_str("zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7").unwrap())
        );
    }
}
