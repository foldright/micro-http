use crate::handler::Handler;
use crate::protocol::{Request, Response};

pub struct SimpleHandler {

}

impl Handler for SimpleHandler {
    fn handle(&self, request: &Request) -> Response {
        todo!()
    }
}