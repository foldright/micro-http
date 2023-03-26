use crate::RequestContext;
use http::{HeaderName, HeaderValue, Method};

pub trait Filter: Send + Sync {
    fn check(&self, req: &RequestContext) -> bool;
}

struct FnFilter<F: Fn(&RequestContext) -> bool>(F);

impl<F: Fn(&RequestContext) -> bool + Send + Sync> Filter for FnFilter<F> {
    fn check(&self, req: &RequestContext) -> bool {
        (self.0)(req)
    }
}
pub fn fn_filter<F>(f: F) -> impl Filter
where
    F: Fn(&RequestContext) -> bool + Send + Sync,
{
    FnFilter(f)
}

pub fn always() -> TrueFilter {
    TrueFilter
}
pub fn always_no() -> FalseFilter {
    FalseFilter
}

pub struct TrueFilter;
impl Filter for TrueFilter {
    #[inline]
    fn check(&self, _req: &RequestContext) -> bool {
        true
    }
}

pub struct FalseFilter;
impl Filter for FalseFilter {
    #[inline]
    fn check(&self, _req: &RequestContext) -> bool {
        false
    }
}

pub fn any_filter() -> AnyFilter {
    AnyFilter::new()
}

/// compose filters with *OR* logic, if any inner filter success, the whole [`AnyFilter`] will success
pub struct AnyFilter {
    filters: Vec<Box<dyn Filter>>,
}

impl AnyFilter {
    fn new() -> Self {
        Self { filters: vec![] }
    }
    pub fn or<F: Filter + 'static>(&mut self, filter: F) -> &mut Self {
        self.filters.push(Box::new(filter));
        self
    }
}

impl Filter for AnyFilter {
    fn check(&self, req: &RequestContext) -> bool {
        if self.filters.is_empty() {
            return true;
        }

        for filter in &self.filters {
            if filter.check(req) {
                return true;
            }
        }

        false
    }
}

pub fn all_filter() -> AllFilter {
    AllFilter::new()
}
/// compose filters with *AND* logic, if any inner filter success, the whole [`AllFilter`] will success
pub struct AllFilter {
    filters: Vec<Box<dyn Filter>>,
}

impl AllFilter {
    fn new() -> Self {
        Self { filters: vec![] }
    }
    pub fn and<F: Filter + 'static>(&mut self, filter: F) -> &mut Self {
        self.filters.push(Box::new(filter));
        self
    }
}

impl Filter for AllFilter {
    fn check(&self, req: &RequestContext) -> bool {
        if self.filters.is_empty() {
            return true;
        }

        for filter in &self.filters {
            if !filter.check(req) {
                return false;
            }
        }

        true
    }
}

pub struct MethodFilter(Method);

impl Filter for MethodFilter {
    fn check(&self, req: &RequestContext) -> bool {
        self.0.eq(req.method())
    }
}

macro_rules! method_filter {
    ($method:ident, $upper_case_method:ident) => {
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

pub struct HeaderFilter(HeaderName, HeaderValue);
impl Filter for HeaderFilter {
    fn check(&self, req: &RequestContext) -> bool {
        let value_option = req.headers().get(&self.0);
        value_option.map(|value| (&self.1).eq(value)).unwrap_or(false)
    }
}
