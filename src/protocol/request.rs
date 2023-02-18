use std::convert::Into;

use http::{HeaderMap, Method, Request, Uri, Version};

#[derive(Debug)]
pub struct RequestHeader {
    inner: Request<()>,
}

impl AsRef<Request<()>> for RequestHeader {
    fn as_ref(&self) -> &Request<()> {
        &self.inner
    }
}

impl AsMut<Request<()>> for RequestHeader {
    fn as_mut(&mut self) -> &mut Request<()> {
        &mut self.inner
    }
}

impl RequestHeader {
    pub fn into_inner(self) -> Request<()> {
        self.inner
    }

    pub fn body<T>(self, body: T) -> Request<T> {
        self.inner.map(|_| body)
    }

    pub fn method(&self) -> &Method {
        self.inner.method()
    }

    pub fn uri(&self) -> &Uri {
        self.inner.uri()
    }

    pub fn version(&self) -> Version {
        self.inner.version()
    }

    pub fn headers(&self) -> &HeaderMap {
        self.inner.headers()
    }

    pub fn need_body(&self) -> bool {
        !matches!(self.method(), &Method::GET | &Method::HEAD | &Method::DELETE | &Method::OPTIONS | &Method::CONNECT)
    }
}

impl<'headers, 'buf> From<httparse::Request<'headers, 'buf>> for RequestHeader {
    fn from(req: httparse::Request<'headers, 'buf>) -> Self {
        let mut builder = Request::builder()
            .method(req.method.unwrap())
            .uri(req.path.unwrap())
            .version(U8Wrapper(req.version.unwrap()).into());

        for header in req.headers.iter() {
            builder = builder.header(header.name, header.value)
        }

        RequestHeader { inner: builder.body(()).unwrap() }
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

    use http::{HeaderValue, Method, Version};
    use indoc::indoc;

    use super::*;

    #[test]
    fn from_curl() {
        let str = indoc! {r##"
        GET /index.html HTTP/1.1
        Host: 127.0.0.1:8080
        User-Agent: curl/7.79.1
        Accept: */*

        "##};

        let mut parsed_req = httparse::Request::new(&mut []);
        let mut headers: [MaybeUninit<httparse::Header>; 4] = unsafe { MaybeUninit::uninit().assume_init() };

        parsed_req.parse_with_uninit_headers(str.as_bytes(), &mut headers).unwrap();

        let header: RequestHeader = parsed_req.into();

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

        let mut parsed_req = httparse::Request::new(&mut []);
        let mut headers: [MaybeUninit<httparse::Header>; 64] = unsafe { MaybeUninit::uninit().assume_init() };

        parsed_req.parse_with_uninit_headers(str.as_bytes(), &mut headers).unwrap();

        let header: RequestHeader = parsed_req.into();

        assert_eq!(header.method(), &Method::GET);
        assert_eq!(header.version(), Version::HTTP_11);
        assert_eq!(header.uri().host(), None);
        assert_eq!(header.uri().path(), "/index/");
        assert_eq!(header.uri().scheme(), None);
        assert_eq!(header.uri().query(), Some("a=1&b=2&a=3"));

        assert_eq!(header.headers().len(), 15);

        // TODO maybe we can using macro to reduce code
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
