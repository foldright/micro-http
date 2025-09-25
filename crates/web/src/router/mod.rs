pub mod filter;

use crate::{handler_fn, FnTrait, PathParams};
use crate::handler::RequestHandler;

use crate::handler::handler_decorator::HandlerDecorator;
use crate::handler::handler_decorator_factory::{
    HandlerDecoratorFactory, HandlerDecoratorFactoryComposer, HandlerDecoratorFactoryExt, IdentityHandlerDecoratorFactory,
};
use filter::{AllFilter, Filter};
use std::collections::HashMap;
use tracing::error;
use crate::extract::FromRequest;
use crate::responder::Responder;

type RouterFilter = dyn Filter + Send + Sync + 'static;
type InnerRouter<T> = matchit::Router<T>;

/// Main router structure that handles HTTP request routing
#[derive(Debug)]
pub struct Router {
    inner_router: InnerRouter<Vec<RouterItem>>,
}

/// A router item containing a filter and handler
#[derive(Debug)]
pub struct RouterItem {
    filter: Box<RouterFilter>,
    handler: Box<dyn RequestHandler>,
}

/// Result of matching a route, containing matched items and path parameters
#[derive(Debug)]
pub struct RouteResult<'router, 'req> {
    router_items: &'router [RouterItem],
    params: PathParams<'router, 'req>,
}

impl Router {
    /// Creates a new router builder with default wrappers
    pub fn builder() -> RouterBuilder<IdentityHandlerDecoratorFactory> {
        RouterBuilder::new()
    }

    /// Matches a path against the router's routes
    ///
    /// Returns a `RouteResult` containing matched handlers and path parameters
    ///
    /// # Arguments
    /// * `path` - The path to match against
    pub fn at<'router, 'req>(&'router self, path: &'req str) -> RouteResult<'router, 'req> {
        self.inner_router
            .at(path)
            .map(|matched| RouteResult { router_items: matched.value.as_slice(), params: matched.params.into() })
            .map_err(|e| error!("match '{}' error: {}", path, e))
            .unwrap_or(RouteResult::empty())
    }
}

impl RouterItem {
    /// Gets the filter for this router item
    pub fn filter(&self) -> &RouterFilter {
        self.filter.as_ref()
    }

    /// Gets the request handler for this router item
    pub fn handler(&self) -> &dyn RequestHandler {
        self.handler.as_ref()
    }
}

impl<'router, 'req> RouteResult<'router, 'req> {
    fn empty() -> Self {
        Self { router_items: &[], params: PathParams::empty() }
    }

    /// Returns true if no routes were matched
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.router_items.is_empty()
    }

    /// Gets the path parameters from the matched route
    pub fn params(&self) -> &PathParams<'router, 'req> {
        &self.params
    }

    /// Gets the matched router items
    pub fn router_items(&self) -> &'router [RouterItem] {
        self.router_items
    }
}

#[derive(Debug)]
pub struct RouterBuilder<DF> {
    data: HashMap<String, Vec<RouterItemBuilder>>,
    decorator_factory: DF,
}

impl RouterBuilder<IdentityHandlerDecoratorFactory> {
    fn new() -> Self {
        Self { data: HashMap::new(), decorator_factory: IdentityHandlerDecoratorFactory }
    }
}
impl<DF> RouterBuilder<DF> {
    pub fn route(mut self, route: impl Into<String>, item_builder: RouterItemBuilder) -> Self {
        let vec = self.data.entry(route.into()).or_default();
        vec.push(item_builder);
        self
    }

    pub fn with_global_decorator<DF2>(self, factory: DF2) -> RouterBuilder<HandlerDecoratorFactoryComposer<DF, DF2>>
    where
        DF: HandlerDecoratorFactory,
        DF2: HandlerDecoratorFactory,
    {
        RouterBuilder { data: self.data, decorator_factory: self.decorator_factory.and_then(factory) }
    }

    /// Builds the router from the accumulated routes and wrappers
    pub fn build(self) -> Router
    where
        DF: HandlerDecoratorFactory,
    {
        let mut inner_router = InnerRouter::new();

        for (path, items) in self.data.into_iter() {
            let router_items = items
                .into_iter()
                .map(|item_builder| item_builder.build())
                .map(|item| {
                    let decorator = self.decorator_factory.create_decorator();
                    let handler = decorator.decorate(item.handler);
                    RouterItem { handler: Box::new(handler), ..item }
                })
                .collect::<Vec<_>>();

            inner_router.insert(path, router_items).unwrap();
        }

        Router { inner_router }
    }
}

macro_rules! inner_method_router_filter {
    ($method:ident, $method_name:ident) => {
        #[inline]
        pub fn $method<H: RequestHandler + 'static>(handler: H) -> RouterItemBuilder {
            let mut filters = filter::all_filter();
            filters.and(filter::$method_name());
            RouterItemBuilder { filters, handler: Box::new(handler) }
        }
    };
}

macro_rules! method_router_filter {
    ($method:ident, $inner_method:ident) => {
        pub fn $method<F, Args>(f: F) -> RouterItemBuilder
        where
            for<'r> F: FnTrait<Args> + 'r,
            for<'r> Args: FromRequest + 'r,
            for<'r> F: FnTrait<Args::Output<'r>>,
            for<'r> <F as FnTrait<Args::Output<'r>>>::Output: Responder,
        {
            let handler = handler_fn(f);
            $inner_method(handler)
        }
    };
}

inner_method_router_filter!(inner_get, get_method);
inner_method_router_filter!(inner_post, post_method);
inner_method_router_filter!(inner_put, put_method);
inner_method_router_filter!(inner_delete, delete_method);
inner_method_router_filter!(inner_head, head_method);
inner_method_router_filter!(inner_options, options_method);
inner_method_router_filter!(inner_connect, connect_method);
inner_method_router_filter!(inner_patch, patch_method);
inner_method_router_filter!(inner_trace, trace_method);

method_router_filter!(get, inner_get);
method_router_filter!(post, inner_post);
method_router_filter!(put, inner_put);
method_router_filter!(delete, inner_delete);
method_router_filter!(head, inner_head);
method_router_filter!(options, inner_options);
method_router_filter!(connect, inner_connect);
method_router_filter!(patch, inner_patch);
method_router_filter!(trace, inner_trace);


#[derive(Debug)]
pub struct RouterItemBuilder {
    filters: AllFilter,
    handler: Box<dyn RequestHandler>,
}

impl RouterItemBuilder {
    pub fn with<F: Filter + Send + Sync + 'static>(mut self, filter: F) -> Self {
        self.filters.and(filter);
        self
    }

    fn build(self) -> RouterItem {
        // todo: we can remove indirect when filters has only one filter
        RouterItem { filter: Box::new(self.filters), handler: self.handler }
    }
}

#[cfg(test)]
mod tests {
    use super::filter::header;
    use super::{Router, get, post};
    use crate::{PathParams, RequestContext};
    use http::{HeaderValue, Method, Request};
    use micro_http::protocol::RequestHeader;

    async fn simple_get_1(_method: &Method) -> String {
        "hello world".into()
    }

    async fn simple_get_2(_method: &Method) -> String {
        "hello world".into()
    }

    fn router() -> Router {
        Router::builder()
            .route("/", get(simple_get_1))
            .route(
                "/",
                post(simple_get_1).with(header(
                    http::header::CONTENT_TYPE,
                    HeaderValue::from_str(mime::APPLICATION_WWW_FORM_URLENCODED.as_ref()).unwrap(),
                )),
            )
            .route("/", post(simple_get_1))
            .route("/2", get(simple_get_2))
            .build()
    }

    #[test]
    fn test_route_get() {
        let router = router();
        let route_result = router.at("/");

        assert_eq!(route_result.params.len(), 0);

        let items = route_result.router_items;
        assert_eq!(items.len(), 3);

        let header: RequestHeader = Request::builder().method(Method::GET).body(()).unwrap().into_parts().0.into();
        let params = PathParams::empty();
        let req_ctx = RequestContext::new(&header, &params);

        assert!(items[0].filter.matches(&req_ctx));
        assert!(!items[1].filter.matches(&req_ctx));
        assert!(!items[2].filter.matches(&req_ctx));
    }

    #[test]
    fn test_route_post() {
        let router = router();
        let route_result = router.at("/");

        assert_eq!(route_result.params.len(), 0);

        let items = route_result.router_items;
        assert_eq!(items.len(), 3);

        let header: RequestHeader = Request::builder().method(Method::POST).body(()).unwrap().into_parts().0.into();
        let params = PathParams::empty();
        let req_ctx = RequestContext::new(&header, &params);

        assert!(!items[0].filter.matches(&req_ctx));
        assert!(!items[1].filter.matches(&req_ctx));
        assert!(items[2].filter.matches(&req_ctx));
    }

    #[test]
    fn test_route_post_with_content_type() {
        let router = router();
        let route_result = router.at("/");

        assert_eq!(route_result.params.len(), 0);

        let items = route_result.router_items;
        assert_eq!(items.len(), 3);

        let header: RequestHeader = Request::builder()
            .method(Method::POST)
            .header(http::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(())
            .unwrap()
            .into_parts()
            .0
            .into();
        let params = PathParams::empty();
        let req_ctx = RequestContext::new(&header, &params);

        assert!(!items[0].filter.matches(&req_ctx));
        assert!(items[1].filter.matches(&req_ctx));
        assert!(items[2].filter.matches(&req_ctx));
    }
}
