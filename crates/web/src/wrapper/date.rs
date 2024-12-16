//! Module for handling HTTP response date headers.
//!
//! This module provides functionality for automatically adding RFC 7231 compliant 
//! date headers to HTTP responses. It implements a wrapper pattern that can be 
//! composed with other wrappers in the request handling pipeline.
//!
//! The main components are:
//! - `DateWrapper`: A wrapper that adds date handling capability
//! - `DateResponseHandler`: The actual handler that adds the Date header to responses
//!
//! The Date header is added according to RFC 7231 Section 7.1.1.2

use crate::date::DateService;
use crate::handler::RequestHandler;
use crate::wrapper::Wrapper;
use crate::{OptionReqBody, RequestContext, ResponseBody};
use async_trait::async_trait;
use http::{HeaderValue, Response};

/// A wrapper that adds automatic date header handling to responses.
///
/// This wrapper creates a `DateResponseHandler` that will add an RFC 7231 compliant
/// Date header to all HTTP responses.
pub struct DateWrapper;

/// A request handler that adds the Date header to responses.
///
/// This handler wraps another handler and adds the Date header to its responses.
/// The Date header is generated using a shared `DateService` instance to avoid
/// unnecessary system calls.
pub struct DateResponseHandler<H: RequestHandler> {
    handler: H,
    // todo: we need to ensure data_service is singleton
    date_service: DateService,
}

impl<H: RequestHandler> Wrapper<H> for DateWrapper {
    type Out = DateResponseHandler<H>;

    fn wrap(&self, handler: H) -> Self::Out {
        DateResponseHandler { handler, date_service: DateService::new() }
    }
}

#[async_trait]
impl<H: RequestHandler> RequestHandler for DateResponseHandler<H> {
    async fn invoke<'server, 'req>(
        &self,
        req: &mut RequestContext<'server, 'req>,
        req_body: OptionReqBody,
    ) -> Response<ResponseBody> {
        let mut resp = self.handler.invoke(req, req_body).await;

        self.date_service.with_http_date(|date_str| {
            resp.headers_mut().insert(
                http::header::DATE,
                HeaderValue::try_from(date_str).unwrap(),
            );
        });

        resp
    }
}
