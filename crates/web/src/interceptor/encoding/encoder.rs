use crate::interceptor::encoding::Writer;
use crate::interceptor::Interceptor;
use crate::{RequestContext, ResponseBody};
use async_trait::async_trait;
use bytes::{Buf, Bytes};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use http::Response;
use http_body::{Body, Frame};
use http_body_util::combinators::UnsyncBoxBody;
use micro_http::protocol::{HttpError, SendError};
use pin_project_lite::pin_project;
use std::fmt::Debug;
use std::io;
use std::io::Write;
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use tracing::{error, trace};

// (almost thanks and) copy from actix-http: https://github.com/actix/actix-web/blob/master/actix-http/src/encoding/encoder.rs

pub(crate) enum Encoder {
    Gzip(ZlibEncoder<Writer>),
}

impl Encoder {
    fn gzip() -> Self {
        Self::Gzip(ZlibEncoder::new(Writer::new(), Compression::best()))
    }

    fn write(&mut self, data: &[u8]) -> Result<(), io::Error> {
        match self {
            Self::Gzip(ref mut encoder) => match encoder.write_all(data) {
                Ok(_) => Ok(()),
                Err(err) => {
                    trace!("Error decoding gzip encoding: {}", err);
                    Err(err)
                }
            },
        }
    }

    fn take(&mut self) -> Bytes {
        match *self {
            Self::Gzip(ref mut encoder) => encoder.get_mut().take(),
        }
    }

    fn finish(self) -> Result<Bytes, io::Error> {
        match self {
            Self::Gzip(encoder) => match encoder.finish() {
                Ok(writer) => Ok(writer.buf.freeze()),
                Err(err) => Err(err),
            },
        }
    }
}

pin_project! {
    struct EncodedBody<B: Body> {
        #[pin]
        inner: B,
        encoder: Option<Encoder>,
        state: Option<bool>,
    }
}

impl<B: Body> EncodedBody<B> {
    fn new(b: B, encoder: Encoder) -> Self {
        Self { inner: b, encoder: Some(encoder), state: Some(true) }
    }
}

impl<B> Body for EncodedBody<B>
where
    B: Body + Unpin,
    B::Data: Buf + Debug,
    B::Error: ToString,
{
    type Data = Bytes;
    type Error = HttpError;

    fn poll_frame(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let mut this = self.project();

        if this.state.is_none() {
            return Poll::Ready(None);
        }

        loop {
            return match ready!(this.inner.as_mut().poll_frame(cx)) {
                Some(Ok(frame)) => {
                    let data = match frame.into_data() {
                        Ok(data) => data,
                        Err(mut frame) => {
                            let debug_info = frame.trailers_mut();
                            error!("want to data from body, but receive trailer header: {:?}", debug_info);
                            return Poll::Ready(Some(Err(SendError::invalid_body(format!(
                                "invalid body frame : {:?}",
                                debug_info
                            ))
                            .into())));
                        }
                    };

                    match this.encoder.as_mut().unwrap().write(data.chunk()) {
                        Ok(_) => (),
                        Err(e) => {
                            return Poll::Ready(Some(Err(SendError::from(e).into())));
                        }
                    }
                    // use wrap here is safe, because we only take it when receive None
                    let bytes = this.encoder.as_mut().unwrap().take();
                    if bytes.is_empty() {
                        continue;
                    }
                    Poll::Ready(Some(Ok(Frame::data(bytes))))
                }
                Some(Err(e)) => Poll::Ready(Some(Err(SendError::invalid_body(e.to_string()).into()))),
                None => {
                    if this.state.is_some() {
                        // will only run below  code once
                        this.state.take();

                        // unwrap here is safe, because we only take once
                        let bytes = match this.encoder.take().unwrap().finish() {
                            Ok(bytes) => bytes,
                            Err(e) => {
                                return Poll::Ready(Some(Err(SendError::from(e).into())));
                            }
                        };
                        if !bytes.is_empty() {
                            Poll::Ready(Some(Ok(Frame::data(bytes))))
                        } else {
                            Poll::Ready(None)
                        }
                    } else {
                        Poll::Ready(None)
                    }
                }
            };
        }
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }
}

pub struct EncodeInterceptor;

#[async_trait]
impl Interceptor for EncodeInterceptor {
    async fn on_response(&self, req: &RequestContext, resp: &mut Response<ResponseBody>) {
        let possible_encodings = req.headers().get(http::header::ACCEPT_ENCODING);
        if possible_encodings.is_none() {
            return;
        }

        //todo we need select encoding
        if !possible_encodings.unwrap().to_str().unwrap().contains("gzip") {
            return;
        }

        let body = resp.body_mut();

        if body.is_empty() {
            return;
        }

        let encoded_body = EncodedBody::new(body.take(), Encoder::gzip());
        body.replace(ResponseBody::stream(UnsyncBoxBody::new(encoded_body)));

        resp.headers_mut().remove(http::header::CONTENT_LENGTH);
        resp.headers_mut().append(http::header::CONTENT_ENCODING, "gzip".parse().unwrap());
    }
}
