use bytes::Bytes;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use futures::executor::block_on;
use http::{Request, Response, StatusCode};
use micro_http::handler::make_handler;
use micro_http::{
    codec::{RequestDecoder, ResponseEncoder},
    connection::HttpConnection,
    protocol::{Message, PayloadSize, ResponseHead, body::ReqBody},
};
use std::{
    error::Error,
    io,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_util::codec::{Decoder, Encoder};

// Mock IO for testing
#[derive(Clone)]
struct MockIO {
    read_data: Vec<u8>,
    write_data: Vec<u8>,
    read_pos: usize,
}

impl MockIO {
    fn new(read_data: Vec<u8>) -> Self {
        Self { read_data, write_data: Vec::new(), read_pos: 0 }
    }
}

impl AsyncRead for MockIO {
    fn poll_read(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let remaining = &self.read_data[self.read_pos..];
        let amt = std::cmp::min(remaining.len(), buf.remaining());
        buf.put_slice(&remaining[..amt]);
        self.read_pos += amt;
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for MockIO {
    fn poll_write(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, io::Error>> {
        self.write_data.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }
}

// Test handler
async fn test_handler(_req: Request<ReqBody>) -> Result<Response<String>, Box<dyn Error + Send + Sync>> {
    let response = Response::builder().status(StatusCode::OK).body("Hello World!".to_string()).unwrap();
    Ok(response)
}

use indoc::indoc;

static REQUEST: &'static str = indoc! {r##"
GET /user/123 HTTP/1.1
Host: 127.0.0.1:3000
Sec-Fetch-Dest: document
Sec-Fetch-Mode: navigate
Sec-Fetch-Site: none
Sec-Fetch-User: ?1
sec-ch-ua: "Not A(Brand";v="8", "Chromium";v="132", "Microsoft Edge";v="132"
sec-ch-ua-mobile: ?0
sec-ch-ua-platform: "macOS"
Cache-Control: max-age=0
Connection: keep-alive
Upgrade-Insecure-Requests: 1
User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36 Edg/132.0.0.0
Accept-Language: zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7
Accept-Encoding: gzip, deflate, br, zstd
Accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7

"##};

fn bench_request_decoder(c: &mut Criterion) {
    c.bench_function("decode_simple_request", |b| {
        b.iter(|| {
            let request = REQUEST.as_bytes();
            let mut decoder = RequestDecoder::new();
            let mut bytes = bytes::BytesMut::from(&request[..]);
            black_box(decoder.decode(&mut bytes).unwrap());
        });
    });
}

fn bench_response_encoder(c: &mut Criterion) {
    let response = Response::builder().status(StatusCode::OK).body("Hello World!".to_string()).unwrap();

    c.bench_function("encode_simple_response", |b| {
        b.iter(|| {
            let mut encoder = ResponseEncoder::new();
            let mut bytes = bytes::BytesMut::new();
            let (header, body) = response.clone().into_parts();
            let payload_size = body.as_bytes().len();
            let message = Message::<_, Bytes>::Header((ResponseHead::from_parts(header, ()), PayloadSize::Length(payload_size as u64)));
            black_box(encoder.encode(message, &mut bytes).unwrap());
        });
    });
}

fn bench_http_connection(c: &mut Criterion) {
    let request = REQUEST.as_bytes();
    let handler = Arc::new(make_handler(test_handler));

    c.bench_function("process_simple_request", |b| {
        b.iter(|| {
            let mock_io = MockIO::new(request.to_vec());
            let (reader, writer) = (mock_io.clone(), mock_io);
            let connection = HttpConnection::new(reader, writer);
            black_box(block_on(connection.process(handler.clone())).unwrap());
        });
    });
}

criterion_group!(benches, bench_request_decoder, bench_response_encoder, bench_http_connection);
criterion_main!(benches);
