use http::{Request, Version};

pub fn parse_request_header<T>(req: httparse::Request) -> crate::Result<Request<Option<T>>> {
    ParsedCompletedHeader(req).try_into()
}

struct ParsedCompletedHeader<'headers, 'buf> (httparse::Request<'headers, 'buf>);

impl<'headers, 'buf, T> TryFrom<ParsedCompletedHeader<'headers, 'buf>> for Request<Option<T>> {
    type Error = crate::Error;

    fn try_from(parsed_completed_header: ParsedCompletedHeader<'headers, 'buf>) -> Result<Self, Self::Error> {
        let req = parsed_completed_header.0;

        let mut builder = Request::builder()
            .method(req.method.unwrap())
            .uri(req.path.unwrap())
            .version(U8Wrapper(req.version.unwrap()).into());


        for header in req.headers.iter() {
            builder = builder.header(header.name, header.value)
        }

        Ok(builder.body(None).unwrap())
    }
}

struct U8Wrapper(u8);

impl From<U8Wrapper> for Version {
    fn from(value: U8Wrapper) -> Self {
        match value.0 {
            1 => Version::HTTP_11,
            0 => Version::HTTP_10,
            // http2 and http3 currently not support
            _ => Version::HTTP_09,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::MaybeUninit;

    use http::{Extensions, HeaderValue, Method, Request};
    use indoc::indoc;

    use crate::protocol::request;

    use super::*;

    #[test]
    fn from_curl() {
        let str = indoc! {r##"
        GET /index.html HTTP/1.1
        Host: 127.0.0.1:8080
        User-Agent: curl/7.79.1
        Accept: */*

        "##};
        let str = format!("{}\r\n{}\r\n{}\r\n{}\r\n\r\n",
                          "GET /index.html HTTP/1.1",
                          "Host: 127.0.0.1:8080",
                          "User-Agent: curl/7.79.1",
                          "Accept: */*",
        );

        let mut parsed_req = httparse::Request::new(&mut []);
        let mut headers: [MaybeUninit<httparse::Header>; 4] =
            unsafe { MaybeUninit::uninit().assume_init() };

        parsed_req.parse_with_uninit_headers(str.as_bytes(), &mut headers).unwrap();

        let request: Request<Option<()>> = ParsedCompletedHeader(parsed_req).try_into().unwrap();

        assert_eq!(request.method(), &Method::GET);
        assert_eq!(request.version(), Version::HTTP_11);
        assert_eq!(request.uri().host(), None);
        assert_eq!(request.uri().path(), "/index.html");
        assert_eq!(request.uri().scheme(), None);
        assert_eq!(request.uri().query(), None);
        assert_eq!(request.body(), &None);

        assert_eq!(request.headers().len(), 3);

        assert_eq!(request.headers().get(http::header::ACCEPT),
                   Some(&HeaderValue::from_str("*/*").unwrap()));

        assert_eq!(request.headers().get(http::header::HOST),
                   Some(&HeaderValue::from_str("127.0.0.1:8080").unwrap()));

        assert_eq!(request.headers().get(http::header::USER_AGENT),
                   Some(&HeaderValue::from_str("curl/7.79.1").unwrap()));

        assert!(request.extensions().is_empty());
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

        let mut parsed_req = httparse::Request::new(&mut []);
        let mut headers: [MaybeUninit<httparse::Header>; 64] =
            unsafe { MaybeUninit::uninit().assume_init() };

        parsed_req.parse_with_uninit_headers(str.as_bytes(), &mut headers).unwrap();

        let request: Request<Option<()>> = ParsedCompletedHeader(parsed_req).try_into().unwrap();


        assert_eq!(request.method(), &Method::GET);
        assert_eq!(request.version(), Version::HTTP_11);
        assert_eq!(request.uri().host(), None);
        assert_eq!(request.uri().path(), "/index/");
        assert_eq!(request.uri().scheme(), None);
        assert_eq!(request.uri().query(), Some("a=1&b=2&a=3"));
        assert_eq!(request.body(), &None);

        assert_eq!(request.headers().len(), 15);

        // TODO maybe we can using macro to reduce code
        assert_eq!(request.headers().get(http::header::CONNECTION),
                   Some(&HeaderValue::from_str("keep-alive").unwrap()));

        assert_eq!(request.headers().get(http::header::CACHE_CONTROL),
                   Some(&HeaderValue::from_str("max-age=0").unwrap()));

        assert_eq!(request.headers().get("sec-ch-ua"),
                   Some(&HeaderValue::from_str(r##""#Not_A Brand";v="99", "Microsoft Edge";v="109", "Chromium";v="109""##).unwrap()));

        assert_eq!(request.headers().get("sec-ch-ua-mobile"),
                   Some(&HeaderValue::from_str("?0").unwrap()));


        assert_eq!(request.headers().get("sec-ch-ua-platform"),
                   Some(&HeaderValue::from_str("\"macOS\"").unwrap()));

        assert_eq!(request.headers().get(http::header::UPGRADE_INSECURE_REQUESTS),
                   Some(&HeaderValue::from_str("1").unwrap()));

        assert_eq!(request.headers().get(http::header::USER_AGENT),
                   Some(&HeaderValue::from_str("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36 Edg/109.0.1518.52").unwrap()));

        assert_eq!(request.headers().get("Sec-Fetch-Site"),
                   Some(&HeaderValue::from_str("none").unwrap()));

        assert_eq!(request.headers().get("Sec-Fetch-Mode"),
                   Some(&HeaderValue::from_str("navigate").unwrap()));

        assert_eq!(request.headers().get("Sec-Fetch-User"),
                   Some(&HeaderValue::from_str("?1").unwrap()));

        assert_eq!(request.headers().get("Sec-Fetch-Dest"),
                   Some(&HeaderValue::from_str("document").unwrap()));

        assert_eq!(request.headers().get(http::header::ACCEPT_ENCODING),
                   Some(&HeaderValue::from_str("gzip, deflate, br").unwrap()));

        assert_eq!(request.headers().get(http::header::ACCEPT_LANGUAGE),
                   Some(&HeaderValue::from_str("zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7").unwrap()));

        assert!(request.extensions().is_empty());
    }
}