use crate::codec::RequestDecoder;
use crate::protocol::{Message, ParseError, PayloadItem, PayloadSize};
use bytes::Bytes;
use futures::{Stream, StreamExt, ready};
use http_body::{Body, Frame, SizeHint};
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::pin::Pin;
use std::ptr::NonNull;
use std::task::{Context, Poll};
use tokio::io::AsyncRead;
use tokio_util::codec::FramedRead;

/// The request body provided to handlers.
///
/// `ReqBody` implements [`Body`] by streaming data directly from the underlying
/// [`FramedRead`] without the intermediate channel indirection that existed in
/// the previous design. The streaming state is owned by a [`ReqBodyState`],
/// while `ReqBody` simply provides the consumer view that is attached to the
/// HTTP request object.
#[derive(Debug)]
pub struct ReqBody {
    inner: ReqBodyRepr,
}

#[derive(Debug)]
enum ReqBodyRepr {
    Streaming(StreamingReqBody),
    NoBody,
}

/// Handle returned alongside [`ReqBody`] that owns the streaming state.
///
/// Once the handler has finished processing the request, the connection calls
/// [`ReqBodyState::finish`] to ensure the body is fully drained and to regain
/// ownership of the [`FramedRead`] so the next request on the connection can be
/// parsed.
#[allow(private_interfaces)]
pub(crate) enum ReqBodyState<R> {
    Streaming(StreamingStateHandle<R>),
    Empty(Option<FramedRead<R, RequestDecoder>>),
}

impl<R> ReqBodyState<R>
where
    R: AsyncRead + Unpin + Send + Debug,
{
    pub(crate) fn new(framed_read: FramedRead<R, RequestDecoder>, payload_size: PayloadSize) -> (ReqBody, ReqBodyState<R>) {
        match payload_size {
            PayloadSize::Empty | PayloadSize::Length(0) => (ReqBody::no_body(), ReqBodyState::Empty(Some(framed_read))),
            _ => {
                let (body, handle) = StreamingStateHandle::new(framed_read, payload_size);
                (body, ReqBodyState::Streaming(handle))
            }
        }
    }

    pub(crate) async fn finish(mut self) -> Result<FramedRead<R, RequestDecoder>, ParseError> {
        match &mut self {
            ReqBodyState::Streaming(handle) => handle.finish().await,
            ReqBodyState::Empty(reader) => Ok(reader.take().expect("ReqBodyState::Empty must contain framed reader")),
        }
    }
}

impl ReqBody {
    pub(crate) fn create_req_body<R>(
        framed_read: FramedRead<R, RequestDecoder>,
        payload_size: PayloadSize,
    ) -> (ReqBody, ReqBodyState<R>)
    where
        R: AsyncRead + Unpin + Send + Debug,
    {
        ReqBodyState::new(framed_read, payload_size)
    }

    fn no_body() -> Self {
        Self { inner: ReqBodyRepr::NoBody }
    }

    fn streaming(streaming: StreamingReqBody) -> Self {
        Self { inner: ReqBodyRepr::Streaming(streaming) }
    }
}

impl Body for ReqBody {
    type Data = Bytes;
    type Error = ParseError;

    fn poll_frame(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();
        match &mut this.inner {
            ReqBodyRepr::Streaming(streaming) => unsafe { streaming.poll_frame(cx) },
            ReqBodyRepr::NoBody => Poll::Ready(None),
        }
    }

    fn is_end_stream(&self) -> bool {
        match &self.inner {
            ReqBodyRepr::Streaming(streaming) => unsafe { streaming.is_end_stream() },
            ReqBodyRepr::NoBody => true,
        }
    }

    fn size_hint(&self) -> SizeHint {
        match &self.inner {
            ReqBodyRepr::Streaming(streaming) => unsafe { streaming.size_hint() },
            ReqBodyRepr::NoBody => SizeHint::with_exact(0),
        }
    }
}

/// Pointer-sized handle shared between the request body object and the owning
/// [`StreamingStateHandle`]. The handle stores function pointers that know how
/// to operate on the concrete streaming state without exposing the generic
/// [`FramedRead`] type in the public [`ReqBody`] API.
#[derive(Clone, Copy)]
struct StreamingReqBody {
    raw: NonNull<()>,
    vtable: &'static ReqBodyVTable,
}

impl Debug for StreamingReqBody {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("StreamingReqBody").field("raw", &self.raw).finish()
    }
}

// SAFETY: `StreamingReqBody` points to the heap-allocated `StreamingState<R>` that
// lives inside `StreamingStateHandle<R>`. The handle outlives any `ReqBody`
// handed to user code and guarantees exclusive access to the state while the
// handler is running. The state itself is `Send` because it owns a
// `FramedRead<R, RequestDecoder>` with `R: Send`, making it safe to move the
// lightweight pointer to other threads.
unsafe impl Send for StreamingReqBody {}

impl StreamingReqBody {
    unsafe fn poll_frame(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Frame<Bytes>, ParseError>>> {
        unsafe { (self.vtable.poll_frame)(self.raw, cx) }
    }

    unsafe fn is_end_stream(&self) -> bool {
        unsafe { (self.vtable.is_end_stream)(self.raw) }
    }

    unsafe fn size_hint(&self) -> SizeHint {
        unsafe { (self.vtable.size_hint)(self.raw) }
    }
}

struct ReqBodyVTable {
    poll_frame: unsafe fn(NonNull<()>, &mut Context<'_>) -> Poll<Option<Result<Frame<Bytes>, ParseError>>>,
    is_end_stream: unsafe fn(NonNull<()>) -> bool,
    size_hint: unsafe fn(NonNull<()>) -> SizeHint,
}

struct StreamingStateHandle<R> {
    state: Option<Box<StreamingState<R>>>,
}

impl<R> StreamingStateHandle<R>
where
    R: AsyncRead + Unpin + Send + Debug,
{
    fn new(framed_read: FramedRead<R, RequestDecoder>, payload_size: PayloadSize) -> (ReqBody, StreamingStateHandle<R>) {
        let mut state = Box::new(StreamingState::new(framed_read, payload_size));
        let raw = NonNull::from(state.as_mut()).cast::<()>();
        let streaming = StreamingReqBody { raw, vtable: StreamingState::<R>::vtable() };
        (ReqBody::streaming(streaming), StreamingStateHandle { state: Some(state) })
    }

    async fn finish(&mut self) -> Result<FramedRead<R, RequestDecoder>, ParseError> {
        let state = self.state.take().expect("StreamingStateHandle::finish called more than once");
        state.finish().await
    }
}

struct StreamingState<R> {
    reader: Option<FramedRead<R, RequestDecoder>>,
    payload_size: PayloadSize,
    reached_eof: bool,
}

impl<R> StreamingState<R>
where
    R: AsyncRead + Unpin + Send + Debug,
{
    fn new(reader: FramedRead<R, RequestDecoder>, payload_size: PayloadSize) -> Self {
        Self { reader: Some(reader), payload_size, reached_eof: false }
    }

    fn vtable() -> &'static ReqBodyVTable {
        &ReqBodyVTable { poll_frame: Self::poll_frame_dyn, is_end_stream: Self::is_end_stream_dyn, size_hint: Self::size_hint_dyn }
    }

    fn reader_pin(&mut self) -> Pin<&mut FramedRead<R, RequestDecoder>> {
        Pin::new(self.reader.as_mut().expect("streaming state reader missing"))
    }

    fn poll_frame(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Frame<Bytes>, ParseError>>> {
        if self.reached_eof {
            return Poll::Ready(None);
        }

        match ready!(self.reader_pin().poll_next(cx)) {
            Some(Ok(Message::Payload(PayloadItem::Chunk(bytes)))) => Poll::Ready(Some(Ok(Frame::data(bytes)))),
            Some(Ok(Message::Payload(PayloadItem::Eof))) => {
                self.reached_eof = true;
                Poll::Ready(None)
            }
            Some(Ok(Message::Header(_))) => {
                self.reached_eof = true;
                Poll::Ready(Some(Err(ParseError::invalid_body("unexpected header while streaming request body"))))
            }
            Some(Err(e)) => {
                self.reached_eof = true;
                Poll::Ready(Some(Err(e)))
            }
            None => {
                self.reached_eof = true;
                Poll::Ready(Some(Err(ParseError::invalid_body("unexpected EOF while streaming request body"))))
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        self.reached_eof
    }

    fn size_hint(&self) -> SizeHint {
        self.payload_size.into()
    }

    async fn finish(mut self) -> Result<FramedRead<R, RequestDecoder>, ParseError> {
        if self.reached_eof {
            return Ok(self.reader.take().expect("streaming state reader missing"));
        }

        let reader = self.reader.as_mut().expect("streaming state reader missing");

        while let Some(message) = reader.next().await {
            match message? {
                Message::Payload(PayloadItem::Chunk(_)) => continue,
                Message::Payload(PayloadItem::Eof) => {
                    self.reached_eof = true;
                    break;
                }
                Message::Header(_) => {
                    self.reached_eof = true;
                    return Err(ParseError::invalid_body("unexpected header while draining request body"));
                }
            }
        }

        if !self.reached_eof {
            return Err(ParseError::invalid_body("connection closed before body EOF"));
        }

        Ok(self.reader.take().expect("streaming state reader missing"))
    }

    unsafe fn poll_frame_dyn(raw: NonNull<()>, cx: &mut Context<'_>) -> Poll<Option<Result<Frame<Bytes>, ParseError>>> {
        // SAFETY: `raw` points to a `StreamingState<R>` stored inside the owning
        // `StreamingStateHandle`. The handle outlives the request body, so the
        // pointer remains valid for the duration of the poll call.
        let state = unsafe { raw.cast::<Self>().as_mut() };
        state.poll_frame(cx)
    }

    unsafe fn is_end_stream_dyn(raw: NonNull<()>) -> bool {
        let state = unsafe { raw.cast::<Self>().as_ref() };
        state.is_end_stream()
    }

    unsafe fn size_hint_dyn(raw: NonNull<()>) -> SizeHint {
        let state = unsafe { raw.cast::<Self>().as_ref() };
        state.size_hint()
    }
}

impl From<PayloadSize> for SizeHint {
    fn from(payload_size: PayloadSize) -> Self {
        match payload_size {
            PayloadSize::Length(length) => SizeHint::with_exact(length),
            PayloadSize::Chunked => SizeHint::new(),
            PayloadSize::Empty => SizeHint::with_exact(0),
        }
    }
}
