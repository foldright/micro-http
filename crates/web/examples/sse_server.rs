use futures::Stream;
use micro_web::encoding::encoder::EncodeDecorator;
use micro_web::responder::sse::{Event, SseStream, build_sse_stream_emitter};
use micro_web::router::{Router, get};
use micro_web::{Server, handler_fn};

async fn sse_process() -> SseStream<impl Stream<Item = Event>> {
    let (stream, mut emitter) = build_sse_stream_emitter(2);

    tokio::spawn(async move {
        for i in 0..5 {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            let _ = emitter.send(Event::from_data(format!("{}", i))).await;
        }

        let _ = emitter.close().await;
    });

    stream
}

async fn default_handler() -> &'static str {
    "404 not found"
}

#[tokio::main]
async fn main() {
    // Build router with multiple routes and handlers
    let router = Router::builder()
        // Basic GET route
        .route("/sse", get(handler_fn(sse_process)))
        // Add response encoding wrapper
        .with_global_decorator(EncodeDecorator)
        .build();

    // Configure and start the server
    Server::builder().router(router).bind("127.0.0.1:8080").default_handler(handler_fn(default_handler)).build().unwrap().start().await;
}
