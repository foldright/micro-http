use micro_web::date::DateServiceDecorator;
use micro_web::responder::Responder;
use micro_web::router::{get, post, Router};
use micro_web::{responder, PathParams, Server};

async fn empty_body() -> &'static str {
    ""
}

async fn echo_uid<'s, 'r>(path_params: &PathParams<'s, 'r>) -> String {
    path_params.get("id").map(|s| s.to_owned()).unwrap()
}

async fn default_handler() -> impl Responder {
    responder::NotFound
}


#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[tokio::main]
async fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();
    
    // Build router with multiple routes and handlers
    let router = Router::builder()
        .route("/", get(empty_body))
        .route("/user", post(empty_body))
        .route("/user/{id}", get(echo_uid))
        .with_global_decorator(DateServiceDecorator)
        .build();

    // Configure and start the server
    Server::builder().router(router).bind("127.0.0.1:3000").default_handler(default_handler).build().unwrap().start().await;
}
