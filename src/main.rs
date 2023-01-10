use std::sync::Arc;
use tiny_http:: SimpleServer;
use tiny_http::handler::SimpleHandler;

fn main() {
    let server = SimpleServer::new("127.0.0.1:8080");
}
