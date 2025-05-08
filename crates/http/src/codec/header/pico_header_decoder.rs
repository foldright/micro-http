use std::ffi::c_int;
use std::os::raw::c_char;
use std::ptr;
use bytes::BytesMut;
use picohttpparser_sys::phr_header;
use tokio_util::codec::Decoder;
use crate::protocol::{ParseError, PayloadSize, RequestHeader};


const MAX_HEADER_BYTES: usize = 8 * 1024;



#[cfg(test)]
mod tests {
    use crate::codec::header::pico_header_decoder::HeaderIndex;
    use bytes::Bytes;
    use http::{HeaderMap, HeaderName, HeaderValue, Method, Uri};
    use indoc::indoc;
    use picohttpparser_sys::phr_header;
    use std::ffi::{CStr, c_int};
    use std::ptr;

    #[test]
    fn from_edge_for_pico() {
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

        let request_bytes = Bytes::copy_from_slice(str.as_bytes());
        use std::os::raw::c_char;
        let cs = request_bytes.as_ptr() as *const c_char;
        let cs_len = request_bytes.len();

        let mut method_ptr: *const c_char = ptr::null();
        let mut method_len: usize = 0;
        let mut path_ptr: *const c_char = ptr::null();
        let mut path_len: usize = 0;
        let mut minor_version: c_int = 0;
        let mut headers = [phr_header::default(); 16];
        let mut num_headers: usize = 16;

        let offset = unsafe {
            picohttpparser_sys::phr_parse_request(
                cs,
                cs_len,
                &mut method_ptr,
                &mut method_len,
                &mut path_ptr,
                &mut path_len,
                &mut minor_version,
                headers.as_mut_ptr(),
                &mut num_headers,
                0,
            )
        };

        let base_ptr = cs as usize;

        let method_start = (method_ptr as usize) - base_ptr;
        let method = Method::from_bytes(&request_bytes[method_start..method_len]).unwrap();

        let path_start = (path_ptr as usize) - base_ptr;
        let uri = Uri::from_maybe_shared(request_bytes.slice(path_start..path_len)).unwrap();

        let mut header_map = HeaderMap::with_capacity(16);

        headers
            .into_iter()
            .filter_map(|header| {
                if header.name.is_null() {
                    None
                } else {
                    let name_start = header.name as usize - base_ptr;
                    let value_start = header.value as usize - base_ptr;

                    let header_name = HeaderName::from_bytes(&request_bytes[name_start..name_start + header.name_len]).unwrap();
                    let header_value = unsafe {
                        HeaderValue::from_maybe_shared_unchecked(request_bytes.slice(value_start..value_start + header.value_len))
                    };
                    Some((header_name, header_value))
                }
            })
            .for_each(|(header_name, header_value)| {
                header_map.insert(header_name, header_value);
            });

        println!("{:?}", header_map);

        println!("{}", offset);
    }
}

#[derive(Clone, Copy)]
struct HeaderIndex {
    /// Start and end byte positions of the header name
    pub(crate) name: (usize, usize),
    /// Start and end byte positions of the header value
    pub(crate) value: (usize, usize),
}
