use crate::responder::Responder;
use crate::{RequestContext, ResponseBody};
use bytes::Bytes;
use futures::channel::mpsc::{SendError, channel};
use futures::{Sink, SinkExt, Stream, StreamExt};
use http::{HeaderValue, Response, StatusCode};
use http_body::Frame;
use http_body_util::StreamBody;
use std::time::Duration;

#[derive(Debug)]
pub struct SseStream<S> {
    stream: S,
}

#[derive(Debug)]
pub struct SseEmitter<S> {
    sink: S,
}

impl<S> SseStream<S>
where
    S: Stream<Item = Event>,
{
    fn new(stream: S) -> Self {
        SseStream { stream }
    }
}

impl<S> SseEmitter<S>
where
    S: Sink<Event, Error = SendError>,
{
    fn new(sink: S) -> Self {
        SseEmitter { sink }
    }
}

impl<S> SseEmitter<S>
where
    S: Sink<Event, Error = SendError> + Unpin,
{
    pub async fn send(&mut self, event: Event) -> Result<(), SendError> {
        self.sink.send(event).await
    }

    pub async fn close(&mut self) -> Result<(), SendError> {
        self.sink.close().await
    }
}

pub fn build_sse_stream_emitter(buffer: usize) -> (SseStream<impl Stream<Item = Event>>, SseEmitter<impl Sink<Event, Error = SendError>>) {
    let (sender, receiver) = channel::<Event>(buffer);
    (SseStream::new(receiver), SseEmitter::new(sender))
}

#[derive(Debug)]
pub enum Event {
    Retry(Duration),
    Message(Message),
}

#[derive(Debug)]
pub struct Message {
    // https://html.spec.whatwg.org/multipage/server-sent-events.html#concept-event-stream-last-event-id
    pub id: Option<String>,
    pub name: Option<String>,
    // the message data
    pub data: String,
}

impl Event {
    pub fn message(data: String, id: Option<String>, name: Option<String>) -> Event {
        Event::Message(Message { id, name, data })
    }

    pub fn from_data(data: String) -> Event {
        Event::Message(Message { id: None, name: None, data })
    }

    pub fn retry(duration: impl Into<Duration>) -> Event {
        Event::Retry(duration.into())
    }
}

impl<S> Responder for SseStream<S>
where
    S: Stream<Item = Event> + Send + 'static,
{
    fn response_to(self, _req: &RequestContext) -> Response<ResponseBody> {
        let mut builder = Response::builder();
        let headers = builder.headers_mut().unwrap();
        headers.reserve(16);
        headers.insert(http::header::CONTENT_TYPE, mime::TEXT_EVENT_STREAM.as_ref().parse().unwrap());
        headers.insert(http::header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        headers.insert(http::header::CONNECTION, HeaderValue::from_static("keep-alive"));

        let event_stream = self.stream.map(|event| match event {
            Event::Message(Message { id, name, data }) => {
                let mut string = String::with_capacity(data.len());

                if let Some(i) = id {
                    string.push_str(&format!("id: {}\n", i));
                }

                if let Some(n) = name {
                    string.push_str(&format!("event: {}\n", n));
                }

                let split = data.lines();

                for s in split {
                    string.push_str(&format!("data: {}\n", s));
                }

                string.push('\n');
                Ok(Frame::data(Bytes::from(string)))
            }
            Event::Retry(duration) => Ok(Frame::data(Bytes::from(format!("retry: {}\n\n", duration.as_millis())))),
        });

        let stream_body = StreamBody::new(event_stream);

        builder.status(StatusCode::OK).body(ResponseBody::stream(stream_body)).unwrap()
    }
}
