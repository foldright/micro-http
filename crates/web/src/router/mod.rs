//! Router module for HTTP request routing and handling.
//!
//! This module provides functionality for routing HTTP requests to appropriate handlers based on paths and filters.
//! It supports method-based routing, path parameters, and middleware-style wrappers.
//!
//! # Examples
//!
//! ```no_run
//! use micro_web::router::{get, Router};
//! use micro_web::handler_fn;
//! async fn hello() -> &'static str {
//!     "Hello, World!"
//! }
//!
//! let router = Router::builder()
//!     .route("/hello", get(handler_fn(hello)))
//!     .build();
//! ```

use crate::handler::RequestHandler;
use crate::PathParams;

use std::collections::HashMap;

use crate::wrapper::{IdentityWrapper, IdentityWrappers, Wrapper, Wrappers};
use tracing::error;
use crate::router::filter::{AllFilter, Filter};

pub mod filter;
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
    pub fn builder() -> RouterBuilder<IdentityWrapper, IdentityWrapper> {
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

/// Builder for constructing a router with routes and wrappers
pub struct RouterBuilder<HeadW, TailW>
where
    HeadW: Wrapper<Box<dyn RequestHandler>> + 'static,
    TailW: Wrapper<HeadW::Out> + 'static,
    TailW::Out: RequestHandler,
{
    data: HashMap<String, Vec<RouterItemBuilder>>,
    wrappers: Wrappers<HeadW, TailW, Box<dyn RequestHandler>>,
}

impl RouterBuilder<IdentityWrapper, IdentityWrapper> {
    fn new() -> Self {
        Self { data: HashMap::new(), wrappers: IdentityWrappers::default() }
    }
}
impl<HeadW, TailW> RouterBuilder<HeadW, TailW>
where
    HeadW: Wrapper<Box<dyn RequestHandler>> + 'static,
    TailW: Wrapper<HeadW::Out> + 'static,
    TailW::Out: RequestHandler,
{
    /// Adds a route to the router builder
    ///
    /// # Arguments
    /// * `route` - The path pattern to match
    /// * `item_builder` - The router item builder containing filters and handler
    pub fn route(mut self, route: impl Into<String>, item_builder: RouterItemBuilder) -> Self {
        let vec = self.data.entry(route.into()).or_default();
        vec.push(item_builder);
        self
    }

    /// Adds a wrapper to the router builder
    ///
    /// Wrappers can modify or enhance the behavior of handlers
    pub fn wrap<NewW>(
        self,
        handler_wrapper: NewW,
    ) -> RouterBuilder<Wrappers<HeadW, TailW, Box<dyn RequestHandler>>, NewW>
    where
        NewW: Wrapper<TailW::Out>,
        NewW::Out: RequestHandler,
    {
        RouterBuilder { data: self.data, wrappers: self.wrappers.and_then(handler_wrapper) }
    }

    /// Builds the router from the accumulated routes and wrappers
    pub fn build(self) -> Router {
        let mut inner_router = InnerRouter::new();

        for (path, items) in self.data.into_iter() {
            let router_items = items
                .into_iter()
                .map(|item_builder| item_builder.build())
                .map(|item| {
                    let handler = self.wrappers.wrap(item.handler);
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
    use crate::router::filter::header;
    use crate::router::{get, post, Router};
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
