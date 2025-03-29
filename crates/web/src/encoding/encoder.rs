use crate::encoding::Writer;
use crate::handler::RequestHandler;
use crate::handler::handler_decorator::HandlerDecorator;
use crate::handler::handler_decorator_factory::HandlerDecoratorFactory;
use crate::{OptionReqBody, RequestContext, ResponseBody};
use async_trait::async_trait;
use bytes::{Buf, Bytes};
use flate2::Compression;
use flate2::write::{GzEncoder, ZlibEncoder};
use http::{Response, StatusCode};
use http_body::{Body, Frame};
use http_body_util::combinators::UnsyncBoxBody;
use micro_http::protocol::{HttpError, SendError};
use pin_project_lite::pin_project;
use std::fmt::Debug;
use std::io;
use std::io::Write;
use std::pin::Pin;
use std::task::{Context, Poll, ready};
use tracing::{error, trace};
use zstd::stream::write::Encoder as ZstdEncoder;
// (almost thanks and) copy from actix-http: https://github.com/actix/actix-web/blob/master/actix-http/src/encoding/encoder.rs

/// Represents different types of content encoding.
pub(crate) enum Encoder {
    /// Gzip encoding.
    Gzip(GzEncoder<Writer>),
    /// Deflate encoding.
    Deflate(ZlibEncoder<Writer>),
    /// Zstd encoding.
    Zstd(ZstdEncoder<'static, Writer>),
    /// Brotli encoding.
    Br(Box<brotli::CompressorWriter<Writer>>),
}

impl Encoder {
    /// Creates a new Gzip encoder.
    fn gzip() -> Self {
        Self::Gzip(GzEncoder::new(Writer::new(), Compression::best()))
    }

    /// Creates a new Deflate encoder.
    fn deflate() -> Self {
        Self::Deflate(ZlibEncoder::new(Writer::new(), Compression::best()))
    }

    /// Creates a new Zstd encoder.
    fn zstd() -> Self {
        // todo: remove the unwrap
        Self::Zstd(ZstdEncoder::new(Writer::new(), 6).unwrap())
    }

    /// Creates a new Brotli encoder.
    fn br() -> Self {
        Self::Br(Box::new(brotli::CompressorWriter::new(
            Writer::new(),
            32 * 1024, // 32 KiB buffer
            3,         // BROTLI_PARAM_QUALITY
            22,        // BROTLI_PARAM_LGWIN
        )))
    }

    /// Selects an encoder based on the `Accept-Encoding` header.
    fn select(accept_encodings: &str) -> Option<Self> {
        if accept_encodings.contains("zstd") {
            Some(Self::zstd())
        } else if accept_encodings.contains("br") {
            Some(Self::br())
        } else if accept_encodings.contains("gzip") {
            Some(Self::gzip())
        } else if accept_encodings.contains("deflate") {
            Some(Self::deflate())
        } else {
            None
        }
    }

    /// Returns the name of the encoding.
    fn name(&self) -> &'static str {
        match self {
            Encoder::Gzip(_) => "gzip",
            Encoder::Deflate(_) => "deflate",
            Encoder::Zstd(_) => "zstd",
            Encoder::Br(_) => "br",
        }
    }

    /// Writes data to the encoder.
    fn write(&mut self, data: &[u8]) -> Result<(), io::Error> {
        match self {
            Self::Gzip(encoder) => match encoder.write_all(data) {
                Ok(_) => Ok(()),
                Err(err) => {
                    trace!("Error encoding gzip encoding: {}", err);
                    Err(err)
                }
            },

            Self::Deflate(encoder) => match encoder.write_all(data) {
                Ok(_) => Ok(()),
                Err(err) => {
                    trace!("Error encoding deflate encoding: {}", err);
                    Err(err)
                }
            },

            Self::Zstd(encoder) => match encoder.write_all(data) {
                Ok(_) => Ok(()),
                Err(err) => {
                    trace!("Error encoding zstd encoding: {}", err);
                    Err(err)
                }
            },

            Self::Br(encoder) => match encoder.write_all(data) {
                Ok(_) => Ok(()),
                Err(err) => {
                    trace!("Error encoding br encoding: {}", err);
                    Err(err)
                }
            },
        }
    }

    /// Takes the encoded data from the encoder.
    fn take(&mut self) -> Bytes {
        match self {
            Self::Gzip(encoder) => encoder.get_mut().take(),
            Self::Deflate(encoder) => encoder.get_mut().take(),
            Self::Zstd(encoder) => encoder.get_mut().take(),
            Self::Br(encoder) => encoder.get_mut().take(),
        }
    }

    /// Finishes the encoding process and returns the encoded data.
    fn finish(self) -> Result<Bytes, io::Error> {
        match self {
            Self::Gzip(encoder) => match encoder.finish() {
                Ok(writer) => Ok(writer.buf.freeze()),
                Err(err) => Err(err),
            },

            Self::Deflate(encoder) => match encoder.finish() {
                Ok(writer) => Ok(writer.buf.freeze()),
                Err(err) => Err(err),
            },

            Self::Zstd(encoder) => match encoder.finish() {
                Ok(writer) => Ok(writer.buf.freeze()),
                Err(err) => Err(err),
            },

            Self::Br(mut encoder) => match encoder.flush() {
                Ok(()) => Ok(encoder.into_inner().buf.freeze()),
                Err(err) => Err(err),
            },
        }
    }
}

pin_project! {
    /// A wrapper around a `Body` that encodes the data.
    struct EncodedBody<B: Body> {
        #[pin]
        inner: B,
        encoder: Option<Encoder>,
        state: Option<bool>,
    }
}

impl<B: Body> EncodedBody<B> {
    /// Creates a new `EncodedBody`.
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
                            return Poll::Ready(Some(
                                Err(SendError::invalid_body(format!("invalid body frame : {:?}", debug_info)).into()),
                            ));
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
                        if !bytes.is_empty() { Poll::Ready(Some(Ok(Frame::data(bytes)))) } else { Poll::Ready(None) }
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

/// A request handler that encodes the response body.
pub struct EncodeRequestHandler<H: RequestHandler> {
    handler: H,
}

/// A wrapper that creates `EncodeRequestHandler`.
pub struct EncodeDecorator;

impl<H: RequestHandler> HandlerDecorator<H> for EncodeDecorator {
    type Output = EncodeRequestHandler<H>;

    fn decorate(&self, raw: H) -> Self::Output {
        EncodeRequestHandler { handler: raw }
    }
}

impl HandlerDecoratorFactory for EncodeDecorator {
    type Output<In>
        = EncodeDecorator
    where
        In: RequestHandler;

    fn create_decorator<In>(&self) -> Self::Output<In>
    where
        In: RequestHandler,
    {
        EncodeDecorator
    }
}

#[async_trait]
impl<H: RequestHandler> RequestHandler for EncodeRequestHandler<H> {
    async fn invoke<'server, 'req>(&self, req: &mut RequestContext<'server, 'req>, req_body: OptionReqBody) -> Response<ResponseBody> {
        let mut resp = self.handler.invoke(req, req_body).await;
        encode(req, &mut resp);
        resp
    }
}

/// Encodes the response body based on the `Accept-Encoding` header.
fn encode(req: &RequestContext, resp: &mut Response<ResponseBody>) {
    let status_code = resp.status();
    if status_code == StatusCode::NO_CONTENT || status_code == StatusCode::SWITCHING_PROTOCOLS {
        return;
    }

    // response has already encoded
    if req.headers().contains_key(http::header::CONTENT_ENCODING) {
        return;
    }

    // request doesn't have any accept encodings
    let possible_encodings = req.headers().get(http::header::ACCEPT_ENCODING);
    if possible_encodings.is_none() {
        return;
    }

    // here using unwrap is safe because we has checked
    let accept_encodings = match possible_encodings.unwrap().to_str() {
        Ok(s) => s,
        Err(_) => {
            return;
        }
    };

    let encoder = match Encoder::select(accept_encodings) {
        Some(encoder) => encoder,
        None => {
            return;
        }
    };

    let body = resp.body_mut();

    if body.is_empty() {
        return;
    }

    match body.size_hint().upper() {
        Some(upper) if upper <= 1024 => {
            // less then 1k, we needn't compress
            return;
        }
        _ => (),
    }

    let encoder_name = encoder.name();
    let encoded_body = EncodedBody::new(body.take(), encoder);
    body.replace(ResponseBody::stream(UnsyncBoxBody::new(encoded_body)));

    resp.headers_mut().remove(http::header::CONTENT_LENGTH);
    resp.headers_mut().append(http::header::CONTENT_ENCODING, encoder_name.parse().unwrap());
}
