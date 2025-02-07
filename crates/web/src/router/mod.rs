pub mod filter;

use crate::handler::RequestHandler;
use crate::PathParams;

use crate::decorator::{Decorator, DecoratorComposer, DecoratorExt, IdentityDecorator};
use filter::{AllFilter, Filter};
use std::collections::HashMap;
use tracing::error;

type RouterFilter = dyn Filter + Send + Sync + 'static;
type InnerRouter<T> = matchit::Router<T>;

/// Main router structure that handles HTTP request routing
pub struct Router {
    inner_router: InnerRouter<Vec<RouterItem>>,
}

/// A router item containing a filter and handler
pub struct RouterItem {
    filter: Box<RouterFilter>,
    handler: Box<dyn RequestHandler>,
}

/// Result of matching a route, containing matched items and path parameters
pub struct RouteResult<'router, 'req> {
    router_items: &'router [RouterItem],
    params: PathParams<'router, 'req>,
}

impl Router {
    /// Creates a new router builder with default wrappers
    pub fn builder() -> RouterBuilder<IdentityDecorator> {
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

pub struct RouterBuilder<D> {
    data: HashMap<String, Vec<RouterItemBuilder>>,
    decorator: D,
}

impl RouterBuilder<IdentityDecorator> {
    fn new() -> Self {
        Self { data: HashMap::new(), decorator: IdentityDecorator }
    }
}
impl<D> RouterBuilder<D> {
    pub fn route(mut self, route: impl Into<String>, item_builder: RouterItemBuilder) -> Self {
        let vec = self.data.entry(route.into()).or_default();
        vec.push(item_builder);
        self
    }

    pub fn with_global_decorator<D2>(self, decorator: D2) -> RouterBuilder<DecoratorComposer<D, D2>>
    where
        D: Decorator<Box<dyn RequestHandler>>,
        D2: Decorator<D::Out>,
    {
        RouterBuilder { data: self.data, decorator: self.decorator.and_then(decorator) }
    }

    /// Builds the router from the accumulated routes and wrappers
    pub fn build(self) -> Router
    where
        D: Decorator<Box<dyn RequestHandler>>,
        <D as Decorator<Box<dyn RequestHandler>>>::Out: RequestHandler + 'static,
    {
        let mut inner_router = InnerRouter::new();

        for (path, items) in self.data.into_iter() {
            let router_items = items
                .into_iter()
                .map(|item_builder| item_builder.build())
                .map(|item| {
                    let handler = self.decorator.decorate(item.handler);
                    RouterItem { handler: Box::new(handler), ..item }
                })
                .collect::<Vec<_>>();

            inner_router.insert(path, router_items).unwrap();
        }

        Router { inner_router }
    }
}

macro_rules! method_router_filter {
    ($method:ident, $method_name:ident) => {
        pub fn $method<H: RequestHandler + 'static>(handler: H) -> RouterItemBuilder {
            let mut filters = filter::all_filter();
            filters.and(filter::$method_name());
            RouterItemBuilder { filters, handler: Box::new(handler) }
        }
    };
}

method_router_filter!(get, get_method);
method_router_filter!(post, post_method);
method_router_filter!(put, put_method);
method_router_filter!(delete, delete_method);
method_router_filter!(head, head_method);
method_router_filter!(options, options_method);
method_router_filter!(connect, connect_method);
method_router_filter!(patch, patch_method);
method_router_filter!(trace, trace_method);

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
    use super::{get, post, Router};
    use crate::{handler_fn, PathParams, RequestContext};
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
            .route("/", get(handler_fn(simple_get_1)))
            .route(
                "/",
                post(handler_fn(simple_get_1)).with(header(
                    http::header::CONTENT_TYPE,
                    HeaderValue::from_str(mime::APPLICATION_WWW_FORM_URLENCODED.as_ref()).unwrap(),
                )),
            )
            .route("/", post(handler_fn(simple_get_1)))
            .route("/2", get(handler_fn(simple_get_2)))
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
