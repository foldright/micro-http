use std::mem::MaybeUninit;

use crate::codec::body::PayloadDecoder;
use bytes::{BytesMut};
use http::{HeaderName, HeaderValue, Request};
use httparse::{Error, Status};
use tokio_util::codec::Decoder;
use tracing::trace;

use crate::ensure;

use crate::protocol::{ParseError, RequestHeader};

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

        let parsed_result = req.parse_with_uninit_headers(src, &mut headers).map_err(|e| match e {
            Error::TooManyHeaders => ParseError::too_many_headers(MAX_HEADER_NUM),
            e => ParseError::invalid_header(e.to_string()),
        });

        match parsed_result? {
            Status::Complete(body_offset) => {
                trace!(body_size = body_offset, "parsed body size");
                ensure!(body_offset <= MAX_HEADER_BYTES, ParseError::too_large_header(body_offset, MAX_HEADER_BYTES));

                // compute the header bytes index
                let mut header_index: [HeaderIndex; MAX_HEADER_NUM] = EMPTY_HEADER_INDEX_ARRAY;
                HeaderIndex::record(src, req.headers, &mut header_index);

                let version = match req.version {
                    Some(0) => http::Version::HTTP_10,
                    Some(1) => http::Version::HTTP_11,
                    // currently not support http2/3
                    _ => return Err(ParseError::InvalidVersion(req.version)),
                };

                let mut header_builder = Request::builder()
                    .method(req.method.ok_or_else(|| ParseError::InvalidMethod)?)
                    .uri(req.path.ok_or_else(|| ParseError::InvalidUri)?)
                    .version(version);

                // build headers

                let header_count = req.headers.len();
                let headers = header_builder.headers_mut().unwrap();
                headers.reserve(header_count);

                let header_bytes = src.split_to(body_offset).freeze();
                for index in &header_index[..header_count] {
                    // it's safe to use unwrap here because httparse has checked the header name is valid ASCII
                    let name = HeaderName::from_bytes(&header_bytes[index.name.0..index.name.1]).unwrap();

                    // inspired by active-web:
                    // SAFETY: httparse already checks header value is only visible ASCII bytes
                    // from_maybe_shared_unchecked contains debug assertions so they are omitted here
                    let value = unsafe {
                        HeaderValue::from_maybe_shared_unchecked(
                            header_bytes.slice(index.value.0..index.value.1),
                        )
                    };

                    headers.append(name, value);
                }

                // build header and parse payload decoder
                let header = RequestHeader::from(header_builder.body(()).unwrap());
                let payload_decoder = parse_payload(&header)?;

                Ok(Some((header, payload_decoder)))
            }
            Status::Partial => {
                ensure!(src.len() <= MAX_HEADER_BYTES, ParseError::too_large_header(src.len(), MAX_HEADER_BYTES));
                Ok(None)
            }
        }
    }
}

#[derive(Clone, Copy)]
struct HeaderIndex {
    pub(crate) name: (usize, usize),
    pub(crate) value: (usize, usize),
}

const EMPTY_HEADER_INDEX: HeaderIndex = HeaderIndex {
    name: (0, 0),
    value: (0, 0),
};

const EMPTY_HEADER_INDEX_ARRAY: [HeaderIndex; MAX_HEADER_NUM] =
    [EMPTY_HEADER_INDEX; MAX_HEADER_NUM];

impl HeaderIndex {
    fn record(
        bytes: &[u8],
        headers: &[httparse::Header<'_>],
        indices: &mut [HeaderIndex],
    ) {
        let bytes_ptr = bytes.as_ptr() as usize;
        for (header, indices) in headers.iter().zip(indices.iter_mut()) {
            let name_start = header.name.as_ptr() as usize - bytes_ptr;
            let name_end = name_start + header.name.len();
            indices.name = (name_start, name_end);
            let value_start = header.value.as_ptr() as usize - bytes_ptr;
            let value_end = value_start + header.value.len();
            indices.value = (value_start, value_end);
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
            let cl_str = cl_value.to_str().map_err(|_| ParseError::invalid_content_length("value can't to_str"))?;

            let length = cl_str
                .trim()
                .parse::<u64>()
                .map_err(|_| ParseError::invalid_content_length(format!("value {cl_str} is not u64")))?;

            Ok(PayloadDecoder::fix_length(length))
        }

        (Some(_), Some(_)) => {
            Err(ParseError::invalid_content_length("transfer_encoding and content_length both present in headers"))
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
