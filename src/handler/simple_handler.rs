use crate::handler::Handler;
use crate::protocol::{Request, Response, StatusCode};

pub struct SimpleHandler {}

impl Handler for SimpleHandler {
    fn handle(&self, request: &Request) -> Response {
        Response::new(StatusCode::OK, Some("it works".into()))
    }
}

impl SimpleHandler {
    pub fn new() -> Self {
        SimpleHandler {}
    }
}