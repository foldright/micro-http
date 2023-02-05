use bytes::{Bytes};
use std::pin::Pin;
use std::task::{Context, Poll, ready};
use anyhow::{anyhow, Context as AnyHowContext};
use futures::StreamExt;
use http::{HeaderMap, HeaderValue};

use http_body::{Body, Frame};
use tokio::io::AsyncRead;
use tokio::net::tcp::OwnedReadHalf;

use crate::protocol::body::BodyLength;
use crate::protocol::RequestHeader;

use tokio_util::codec::{FramedRead};

use crate::codec::{BodyData, BodyDecoder};
use crate::Error;


pub struct ReqBody<T: AsyncRead + Unpin = OwnedReadHalf> {
    pub framed_read: FramedRead<T, BodyDecoder>,
}

impl ReqBody {
    pub fn new(framed_read: FramedRead<OwnedReadHalf, BodyDecoder>) -> Self {
        Self {
            framed_read,
        }
    }

    pub fn parse_body_length(header: &RequestHeader) -> crate::Result<BodyLength> {
        if !header.need_body() {
            return Ok(BodyLength::empty());
        }

        // refer: https://www.rfc-editor.org/rfc/rfc7230#section-3.3
        let te_header = header.headers().get(http::header::TRANSFER_ENCODING);
        let cl_header = header.headers().get(http::header::CONTENT_LENGTH);

        match (te_header, cl_header) {
            (None, None) => Ok(BodyLength::empty()),
            (te_value @ Some(_), None) => {
                if Self::is_chunked_from(te_value) {
                    Ok(BodyLength::chunked())
                } else {
                    Ok(BodyLength::empty())
                }
            }
            (None, Some(cl_value)) => cl_value
                .to_str()
                .ok()
                .and_then(|str| str.trim().parse::<usize>().ok())
                .map(|length| BodyLength::fix(length))
                .with_context(|| format!("content-length is illegal: {:?}", cl_value)),

            (Some(_), Some(_)) => {
                Err(Error::msg("transfer_encoding and content_length both present in headers".to_string()))

            }
        }
    }

    /// tests if headers declared chunked in transfer_encoding
    fn is_chunked(headers: &HeaderMap) -> bool {
        Self::is_chunked_from(headers.get(http::header::TRANSFER_ENCODING))
    }

    fn is_chunked_from(header_value: Option<&HeaderValue>) -> bool {
        header_value
            .and_then(|value| value.to_str().ok())
            .and_then(|encodings| encodings.rsplit(',').next())
            .map(|last_encoding| last_encoding.trim() == "chunked")
            .unwrap_or(false)
    }
}

impl Body for ReqBody where {
    type Data = Bytes;
    type Error = crate::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let framed_read = &mut self.framed_read;
        match ready!(framed_read.poll_next_unpin(cx)) {
            Some(Ok(item)) => {
                match item {
                    BodyData::Bytes(bytes) => Poll::Ready(Some(Ok(Frame::data(bytes)))),
                    BodyData::Finished => Poll::Ready(None),
                }
            }
            None | Some(Err(_)) => {
                Poll::Ready(Some(Err(anyhow!("read body error"))))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderMap;

    #[test]
    fn check_is_chunked() {
        {
            let headers = HeaderMap::new();
            assert!(!ReqBody::is_chunked(&headers))
        }

        {
            let mut headers = HeaderMap::new();
            headers.insert("Accept", "foo".parse().unwrap());
            headers.insert("Transfer-Encoding", "gzip, chunked".parse().unwrap());
            headers.insert("Host", "bar".parse().unwrap());
            assert!(ReqBody::is_chunked(&headers))
        }

        {
            let mut headers = HeaderMap::new();
            headers.insert("Accept", "foo".parse().unwrap());
            headers.insert("Transfer-Encoding", "chunked, gzip".parse().unwrap());
            headers.insert("Host", "bar".parse().unwrap());
            assert!(!ReqBody::is_chunked(&headers))
        }

        {
            let mut headers = HeaderMap::new();
            headers.insert("Accept", "foo".parse().unwrap());
            headers.insert("Transfer-Encoding", "gzip".parse().unwrap());
            headers.insert("Host", "bar".parse().unwrap());
            assert!(!ReqBody::is_chunked(&headers))
        }
    }
}
