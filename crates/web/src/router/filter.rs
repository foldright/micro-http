//! Request filtering module that provides composable request filters.
//!
//! This module implements a filter system that allows you to:
//! - Filter requests based on HTTP methods
//! - Filter requests based on headers
//! - Combine multiple filters using AND/OR logic
//! - Create custom filters using closures
//!
//! ## Thread Safety
//!
//! All filters must implement the `Filter` trait, which requires `Send + Sync`.
//! This ensures that filters can be safely shared and used across threads,
//! which is essential for concurrent request handling in a web server environment.
//!
//! # Examples
//!
//! ```
//! use micro_web::router::filter::{all_filter, any_filter, get_method, header};
//!
//! // Create a filter that matches GET requests
//! let get_filter = get_method();
//!
//! // Create a filter that checks for specific header
//! let auth_filter = header("Authorization", "Bearer token");
//!
//! // Combine filters with AND logic
//! let mut combined = all_filter();
//! combined.and(get_filter).and(auth_filter);
//! ```

use crate::RequestContext;
use http::{HeaderName, HeaderValue, Method};

/// Core trait for request filtering.
///
/// Implementors of this trait can be used to filter HTTP requests
/// based on custom logic. Filters can be composed using [`AllFilter`]
/// and [`AnyFilter`].
///
///
/// The `Filter` trait requires `Send + Sync`, ensuring that filters
/// can be safely used in a multi-threaded environment.
pub trait Filter: Send + Sync {
    /// Check if the request matches this filter's criteria.
    ///
    /// Returns `true` if the request should be allowed, `false` otherwise.
    fn matches(&self, req: &RequestContext) -> bool;
}

/// A filter that wraps a closure.
struct FnFilter<F: Fn(&RequestContext) -> bool>(F);

impl<F: Fn(&RequestContext) -> bool + Send + Sync> Filter for FnFilter<F> {
    fn matches(&self, req: &RequestContext) -> bool {
        (self.0)(req)
    }
}

/// Creates a new filter from a closure.
///
/// This allows creating custom filters using simple closures.
///
/// # Example
/// ```
/// use micro_web::router::filter::fn_filter;
///
/// let custom_filter = fn_filter(|req| {
///     req.uri().path().starts_with("/api")
/// });
/// ```
pub fn fn_filter<F>(f: F) -> impl Filter
where
    F: Fn(&RequestContext) -> bool + Send + Sync,
{
    FnFilter(f)
}

/// Creates a filter that always returns true.
pub fn true_filter() -> TrueFilter {
    TrueFilter
}

/// Creates a filter that always returns false.
pub fn false_filter() -> FalseFilter {
    FalseFilter
}

/// A filter that always returns true.
pub struct TrueFilter;
impl Filter for TrueFilter {
    #[inline]
    fn matches(&self, _req: &RequestContext) -> bool {
        true
    }
}

/// A filter that always returns false.
pub struct FalseFilter;
impl Filter for FalseFilter {
    #[inline]
    fn matches(&self, _req: &RequestContext) -> bool {
        false
    }
}

/// Creates a new OR-composed filter chain.
pub fn any_filter() -> AnyFilter {
    AnyFilter::new()
}

/// Compose filters with OR logic.
///
/// If any inner filter succeeds, the whole filter succeeds.
/// An empty filter chain returns true by default.
pub struct AnyFilter {
    filters: Vec<Box<dyn Filter>>,
}

impl AnyFilter {
    fn new() -> Self {
        Self { filters: vec![] }
    }

    /// Add a new filter to the OR chain.
    pub fn or<F: Filter + 'static>(&mut self, filter: F) -> &mut Self {
        self.filters.push(Box::new(filter));
        self
    }
}

impl Filter for AnyFilter {
    fn matches(&self, req: &RequestContext) -> bool {
        if self.filters.is_empty() {
            return true;
        }

        for filter in &self.filters {
            if filter.matches(req) {
                return true;
            }
        }

        false
    }
}

/// Creates a new AND-composed filter chain.
pub fn all_filter() -> AllFilter {
    AllFilter::new()
}

/// Compose filters with AND logic.
///
/// All inner filters must succeed for the whole filter to succeed.
/// An empty filter chain returns true by default.
pub struct AllFilter {
    filters: Vec<Box<dyn Filter>>,
}

impl AllFilter {
    fn new() -> Self {
        Self { filters: vec![] }
    }

    /// Add a new filter to the AND chain.
    pub fn and<F: Filter + 'static>(&mut self, filter: F) -> &mut Self {
        self.filters.push(Box::new(filter));
        self
    }
}

impl Filter for AllFilter {
    fn matches(&self, req: &RequestContext) -> bool {
        if self.filters.is_empty() {
            return true;
        }

        for filter in &self.filters {
            if !filter.matches(req) {
                return false;
            }
        }

        true
    }
}

/// A filter that matches HTTP methods.
pub struct MethodFilter(Method);

impl Filter for MethodFilter {
    fn matches(&self, req: &RequestContext) -> bool {
        self.0.eq(req.method())
    }
}

macro_rules! method_filter {
    ($method:ident, $upper_case_method:ident) => {
        #[doc = concat!("Creates a filter that matches HTTP ", stringify!($upper_case_method), " requests.")]
        #[inline]
        pub fn $method() -> MethodFilter {
            MethodFilter(Method::$upper_case_method)
        }
    };
}

method_filter!(get_method, GET);
method_filter!(post_method, POST);
method_filter!(put_method, PUT);
method_filter!(delete_method, DELETE);
method_filter!(head_method, HEAD);
method_filter!(options_method, OPTIONS);
method_filter!(connect_method, CONNECT);
method_filter!(patch_method, PATCH);
method_filter!(trace_method, TRACE);

/// Creates a filter that matches a specific header name and value.
#[inline]
pub fn header<K, V>(header_name: K, header_value: V) -> HeaderFilter
where
    HeaderName: TryFrom<K>,
    <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
    HeaderValue: TryFrom<V>,
    <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
{
    // TODO: need to process the unwrap
    let name = <HeaderName as TryFrom<K>>::try_from(header_name).map_err(Into::into).unwrap();
    let value = <HeaderValue as TryFrom<V>>::try_from(header_value).map_err(Into::into).unwrap();
    HeaderFilter(name, value)
}

/// A filter that matches HTTP headers.
pub struct HeaderFilter(HeaderName, HeaderValue);

impl Filter for HeaderFilter {
    fn matches(&self, req: &RequestContext) -> bool {
        let value_option = req.headers().get(&self.0);
        value_option.map(|value| self.1.eq(value)).unwrap_or(false)
    }
}
