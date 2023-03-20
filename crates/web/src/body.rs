use bytes::Bytes;
use http_body::Body as HttpBody;
use http_body::{Frame, SizeHint};
use http_body_util::combinators::UnsyncBoxBody;
use micro_http::protocol::body::ReqBody;
use micro_http::protocol::{HttpError, ParseError};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct OptionReqBody {
    inner: Arc<Mutex<Option<ReqBody>>>,
}

impl From<ReqBody> for OptionReqBody {
    fn from(body: ReqBody) -> Self {
        OptionReqBody { inner: Arc::new(Mutex::new(Some(body))) }
    }
}

impl OptionReqBody {
    pub async fn can_consume(&self) -> bool {
        let guard = self.inner.lock().await;
        guard.is_some()
    }

    pub async fn apply<T, F, Fut>(&self, f: F) -> Fut::Output
    where
        F: FnOnce(ReqBody) -> Fut,
        Fut: Future<Output = Result<T, ParseError>>,
    {
        let mut guard = self.inner.lock().await;
        if guard.is_none() {
            return Err(ParseError::invalid_body("body has been consumed"));
        }

        let req_body = (*guard).take().unwrap();

        f(req_body).await
    }
}

pub struct ResponseBody {
    inner: Kind,
}

enum Kind {
    Once(Option<Bytes>),
    Stream(UnsyncBoxBody<Bytes, HttpError>),
}

impl ResponseBody {
    pub fn empty() -> Self {
        Self { inner: Kind::Once(None) }
    }

    pub fn once(bytes: Bytes) -> Self {
        Self { inner: Kind::Once(Some(bytes)) }
    }

    pub fn stream<B>(body: B) -> Self
    where
        B: HttpBody<Data = Bytes, Error = HttpError> + Send + 'static,
    {
        Self { inner: Kind::Stream(UnsyncBoxBody::new(body)) }
    }
}

impl From<String> for ResponseBody {
    fn from(value: String) -> Self {
        ResponseBody { inner: Kind::Once(Some(Bytes::from(value))) }
    }
}

impl From<()> for ResponseBody {
    fn from(_: ()) -> Self {
        Self::empty()
    }
}

impl From<Option<Bytes>> for ResponseBody {
    fn from(option: Option<Bytes>) -> Self {
        match option {
            Some(bytes) => Self::once(bytes),
            None => Self::empty(),
        }
    }
}

impl From<&'static str> for ResponseBody {
    fn from(value: &'static str) -> Self {
        if value.is_empty() {
            Self::empty()
        } else {
            Self::once(value.as_bytes().into())
        }
    }
}

impl HttpBody for ResponseBody {
    type Data = Bytes;
    type Error = HttpError;

    fn poll_frame(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let kind = &mut self.get_mut().inner;
        match kind {
            Kind::Once(option_bytes) if option_bytes.is_none() => Poll::Ready(None),
            Kind::Once(option_bytes) => Poll::Ready(Some(Ok(Frame::data(option_bytes.take().unwrap())))),
            Kind::Stream(box_body) => {
                let pin = Pin::new(box_body);
                pin.poll_frame(cx)
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        let kind = &self.inner;
        match kind {
            Kind::Once(option_bytes) => option_bytes.is_none(),
            Kind::Stream(box_body) => box_body.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        let kind = &self.inner;
        match kind {
            Kind::Once(None) => SizeHint::with_exact(0),
            Kind::Once(Some(bytes)) => SizeHint::with_exact(bytes.len() as u64),
            Kind::Stream(box_body) => box_body.size_hint(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::body::ResponseBody;
    use bytes::Bytes;
    use futures::TryStreamExt;
    use http_body::{Body as HttpBody, Frame};
    use http_body_util::{BodyExt, StreamBody};
    use std::io;
    use micro_http::protocol::ParseError;

    fn check_send<T: Send>() {}

    #[test]
    fn is_send() {
        check_send::<ResponseBody>();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_string_body() {
        let s = "Hello world".to_string();
        let len = s.len() as u64;

        let mut body = ResponseBody::from(s);

        assert_eq!(body.size_hint().exact(), Some(len));
        assert_eq!(body.is_end_stream(), false);

        let bytes = body.frame().await.unwrap().unwrap().into_data().unwrap();
        assert_eq!(bytes, Bytes::from("Hello world"));

        assert_eq!(body.is_end_stream(), true);
        assert!(body.frame().await.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_empty_body() {
        let mut body = ResponseBody::from("");

        assert_eq!(body.is_end_stream(), true);
        assert_eq!(body.size_hint().exact(), Some(0));

        assert!(body.frame().await.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_stream_body() {
        let chunks: Vec<Result<_, io::Error>> = vec![
            Ok(Frame::data(Bytes::from(vec![1]))),
            Ok(Frame::data(Bytes::from(vec![2]))),
            Ok(Frame::data(Bytes::from(vec![3]))),
        ];
        let stream = futures::stream::iter(chunks).map_err(|err|ParseError::io(err).into());
        let stream_body = StreamBody::new(stream);

        let mut body = ResponseBody::stream(stream_body);

        assert!(body.size_hint().exact().is_none());
        assert_eq!(body.is_end_stream(), false);
        assert_eq!(body.frame().await.unwrap().unwrap().into_data().unwrap().as_ref(), [1]);
        assert_eq!(body.frame().await.unwrap().unwrap().into_data().unwrap().as_ref(), [2]);
        assert_eq!(body.frame().await.unwrap().unwrap().into_data().unwrap().as_ref(), [3]);

        assert_eq!(body.is_end_stream(), false);

        assert!(body.frame().await.is_none());

        assert_eq!(body.is_end_stream(), false);
    }
}
