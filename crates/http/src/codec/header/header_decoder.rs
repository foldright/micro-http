//! HTTP header decoder implementation for parsing HTTP request headers
//!
//! This module provides functionality for decoding HTTP request headers from raw bytes into
//! structured header representations. It handles parsing of HTTP method, URI, version and
//! header fields according to HTTP/1.1 specification.
//!
//! # Features
//!
//! - Efficient zero-copy header parsing using `httparse`
//! - Support for HTTP/1.0 and HTTP/1.1
//! - Memory safety through `MaybeUninit` for header allocation
//! - Built-in protection against oversized headers
//! - Automatic payload decoder selection based on headers
//!
//! # Limits
//!
//! - Maximum number of headers: 64
//! - Maximum header size: 8KB
//! - Only supports HTTP/1.0 and HTTP/1.1 (HTTP/2 and HTTP/3 currently not supported)
//!
//! # Implementation Details
//!
//! The decoder works in multiple stages:
//!
//! 1. Parse raw bytes using `httparse`
//! 2. Record header name/value byte ranges
//! 3. Convert to typed `http::Request` structure
//! 4. Determine payload decoder based on headers
//!
//! The implementation uses an index-based approach to avoid copying header data,
//! recording the byte ranges of header names and values for efficient conversion
//! to the final header structure.

use std::mem::MaybeUninit;

use bytes::BytesMut;
use http::{HeaderName, HeaderValue, Request};
use httparse::{Error, Status};
use tokio_util::codec::Decoder;
use tracing::trace;

use crate::ensure;

use crate::protocol::{ParseError, PayloadSize, RequestHeader};

/// Maximum number of headers allowed in a request
const MAX_HEADER_NUM: usize = 64;

/// Maximum size in bytes allowed for the entire header section
const MAX_HEADER_BYTES: usize = 8 * 1024;

/// Decoder for HTTP request headers implementing the [`Decoder`] trait.
///
/// This decoder parses raw bytes into a structured [`RequestHeader`] and determines the
/// appropriate [`PayloadDecoder`] based on the Content-Length and Transfer-Encoding headers.
pub struct HeaderDecoder;

impl Decoder for HeaderDecoder {
    type Item = (RequestHeader, PayloadSize);
    type Error = ParseError;

    /// Attempts to decode HTTP headers from the provided bytes buffer.
    ///
    /// # Arguments
    ///
    /// * `src` - Mutable reference to the source bytes buffer
    ///
    /// # Returns
    ///
    /// - `Ok(Some((header, decoder)))` if a complete header was successfully parsed
    /// - `Ok(None)` if more data is needed
    /// - `Err(ParseError)` if parsing failed
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if:
    /// - The number of headers exceeds `MAX_HEADER_NUM`
    /// - The total header size exceeds `MAX_HEADER_BYTES`
    /// - The HTTP version is not supported
    /// - Headers contain invalid characters
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Fast path: return early if buffer is empty or too small
        if src.len() < 14 {  // Minimum valid HTTP request needs at least "GET / HTTP/1.1\r\n\r\n"
            return Ok(None);
        }
        
        // Create an empty HTTP request parser and uninitialized headers array
        let mut req = httparse::Request::new(&mut []);
        let mut headers: [MaybeUninit<httparse::Header>; MAX_HEADER_NUM] = unsafe { MaybeUninit::uninit().assume_init() };

        // Parse request headers using httparse, return error if exceeds max headers or invalid format
        let parsed_result = req.parse_with_uninit_headers(src, &mut headers).map_err(|e| match e {
            Error::TooManyHeaders => ParseError::too_many_headers(MAX_HEADER_NUM),
            e => ParseError::invalid_header(e.to_string()),
        });

        match parsed_result? {
            // If parsing is complete, get the body offset
            Status::Complete(body_offset) => {
                trace!(body_size = body_offset, "parsed body size");
                // Ensure request headers size does not exceed limit
                ensure!(body_offset <= MAX_HEADER_BYTES, ParseError::too_large_header(body_offset, MAX_HEADER_BYTES));

                let header_count = req.headers.len();

                ensure!(header_count <= MAX_HEADER_NUM, ParseError::too_many_headers(header_count));

                // Calculate and record byte range indices for each header
                let mut header_index: [HeaderIndex; MAX_HEADER_NUM] = EMPTY_HEADER_INDEX_ARRAY;
                HeaderIndex::record(src, req.headers, &mut header_index);

                // Build HTTP version based on version number
                let version = match req.version {
                    Some(0) => http::Version::HTTP_10,
                    Some(1) => http::Version::HTTP_11,
                    // Currently HTTP/2 and HTTP/3 not supported
                    _ => return Err(ParseError::InvalidVersion(req.version)),
                };

                // Build request header using parsed method, URI and version
                let mut header_builder = Request::builder()
                    .method(req.method.ok_or(ParseError::InvalidMethod)?)
                    .uri(req.path.ok_or(ParseError::InvalidUri)?)
                    .version(version);

                // Build headers
                let headers = header_builder.headers_mut().unwrap();
                headers.reserve(header_count);

                // Split header portion from source buffer
                let header_bytes = src.split_to(body_offset).freeze();
                // Iterate header indices and build each header
                for index in &header_index[..header_count] {
                    // Safe to unwrap since httparse verified header name is valid ASCII
                    let name = HeaderName::from_bytes(&header_bytes[index.name.0..index.name.1]).unwrap();

                    // inspired by active-web:
                    // Safe to use from_maybe_shared_unchecked since httparse verified
                    // header value contains only visible ASCII chars
                    let value = unsafe { HeaderValue::from_maybe_shared_unchecked(header_bytes.slice(index.value.0..index.value.1)) };

                    headers.append(name, value);
                }

                // Build final request header and payload decoder
                let header = RequestHeader::from(header_builder.body(()).unwrap());
                let payload_decoder = parse_payload(&header)?;

                Ok(Some((header, payload_decoder)))
            }
            // If parsing incomplete, ensure current buffer size does not exceed limit
            Status::Partial => {
                ensure!(src.len() <= MAX_HEADER_BYTES, ParseError::too_large_header(src.len(), MAX_HEADER_BYTES));
                Ok(None)
            }
        }
    }
}

/// Stores the byte range positions of a header's name and value within the original buffer.
///
/// This struct is used internally by the decoder to perform zero-copy parsing of headers
/// by recording the positions of header names and values rather than copying the data.
#[derive(Clone, Copy)]
struct HeaderIndex {
    /// Start and end byte positions of the header name
    pub(crate) name: (usize, usize),
    /// Start and end byte positions of the header value
    pub(crate) value: (usize, usize),
}

const EMPTY_HEADER_INDEX: HeaderIndex = HeaderIndex { name: (0, 0), value: (0, 0) };

const EMPTY_HEADER_INDEX_ARRAY: [HeaderIndex; MAX_HEADER_NUM] = [EMPTY_HEADER_INDEX; MAX_HEADER_NUM];

impl HeaderIndex {
    /// Records the byte positions of header names and values from the parsed headers.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The original bytes containing the headers
    /// * `headers` - Slice of parsed header references from httparse
    /// * `indices` - Mutable slice to store the recorded positions
    fn record(bytes: &[u8], headers: &[httparse::Header<'_>], indices: &mut [HeaderIndex]) {
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

/// Determines the appropriate payload decoder based on the request headers.
///
/// This function examines the Content-Length and Transfer-Encoding headers
/// to select the correct decoder according to RFC 7230 section 3.3.
///
/// # Arguments
///
/// * `header` - The parsed request header
///
/// # Returns
///
/// Returns a `PayloadDecoder` configured based on the headers:
/// - Empty decoder if no body is expected
/// - Chunked decoder if Transfer-Encoding: chunked is present
/// - Fixed-length decoder if Content-Length is present
///
/// # Errors
///
/// Returns `ParseError` if:
/// - Both Content-Length and Transfer-Encoding headers are present
/// - Content-Length value is invalid
fn parse_payload(header: &RequestHeader) -> Result<PayloadSize, ParseError> {
    if !header.need_body() {
        return Ok(PayloadSize::new_empty());
    }

    // refer: https://www.rfc-editor.org/rfc/rfc9112.html#name-transfer-encoding
    let te_header = header.headers().get(http::header::TRANSFER_ENCODING);
    let cl_header = header.headers().get(http::header::CONTENT_LENGTH);

    match (te_header, cl_header) {
        (None, None) => Ok(PayloadSize::new_empty()),

        (te_value @ Some(_), None) => {
            if is_chunked(te_value) {
                Ok(PayloadSize::new_chunked())
            } else {
                Ok(PayloadSize::new_empty())
            }
        }

        (None, Some(cl_value)) => {
            let cl_str = cl_value.to_str().map_err(|_| ParseError::invalid_content_length("value can't to_str"))?;

            let length =
                cl_str.trim().parse::<u64>().map_err(|_| ParseError::invalid_content_length(format!("value {cl_str} is not u64")))?;

            Ok(PayloadSize::new_length(length))
        }

        (Some(_), Some(_)) => Err(ParseError::invalid_content_length("transfer_encoding and content_length both present in headers")),
    }
}

/// Checks if the Transfer-Encoding header indicates chunked encoding.
///
/// According to RFC 7230, chunked must be the last encoding if present.
///
/// # Arguments
///
/// * `header_value` - Optional reference to the Transfer-Encoding header value
///
/// # Returns
///
/// Returns true if chunked is the final encoding in the Transfer-Encoding header.
fn is_chunked(header_value: Option<&HeaderValue>) -> bool {
    const CHUNKED: &[u8] = b"chunked";
    if let Some(value) = header_value {
        if let Some(bytes) = value.as_bytes().rsplit(|b| *b == b',').next() {
            return bytes.trim_ascii() == CHUNKED;
        }
    }
    false
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

        assert_eq!(header.headers().get(http::header::USER_AGENT), Some(&HeaderValue::from_str("curl/7.79.1").unwrap()));
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

        assert_eq!(header.headers().get(http::header::CACHE_CONTROL), Some(&HeaderValue::from_str("max-age=0").unwrap()));

        assert_eq!(
            header.headers().get("sec-ch-ua"),
            Some(&HeaderValue::from_str(r##""#Not_A Brand";v="99", "Microsoft Edge";v="109", "Chromium";v="109""##).unwrap())
        );

        assert_eq!(header.headers().get("sec-ch-ua-mobile"), Some(&HeaderValue::from_str("?0").unwrap()));

        assert_eq!(header.headers().get("sec-ch-ua-platform"), Some(&HeaderValue::from_str("\"macOS\"").unwrap()));

        assert_eq!(header.headers().get(http::header::UPGRADE_INSECURE_REQUESTS), Some(&HeaderValue::from_str("1").unwrap()));

        assert_eq!(header.headers().get(http::header::USER_AGENT),
                   Some(&HeaderValue::from_str("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36 Edg/109.0.1518.52").unwrap()));

        assert_eq!(header.headers().get("Sec-Fetch-Site"), Some(&HeaderValue::from_str("none").unwrap()));

        assert_eq!(header.headers().get("Sec-Fetch-Mode"), Some(&HeaderValue::from_str("navigate").unwrap()));

        assert_eq!(header.headers().get("Sec-Fetch-User"), Some(&HeaderValue::from_str("?1").unwrap()));

        assert_eq!(header.headers().get("Sec-Fetch-Dest"), Some(&HeaderValue::from_str("document").unwrap()));

        assert_eq!(header.headers().get(http::header::ACCEPT_ENCODING), Some(&HeaderValue::from_str("gzip, deflate, br").unwrap()));

        assert_eq!(
            header.headers().get(http::header::ACCEPT_LANGUAGE),
            Some(&HeaderValue::from_str("zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7").unwrap())
        );
    }
}
