use std::sync::Arc;
use tiny_http::handler::SimpleHandler;
use tiny_http::server::SimpleServer;

fn main() {
    let server = SimpleServer::new("127.0.0.1:8080");
    server.run(Arc::new(SimpleHandler::new())).unwrap();
}
