use crate::date::DateService;
use crate::handler::RequestHandler;
use crate::wrapper::Wrapper;
use crate::{OptionReqBody, RequestContext, ResponseBody};
use async_trait::async_trait;
use http::{HeaderValue, Response};

pub struct DateWrapper;

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
