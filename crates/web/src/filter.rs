use crate::RequestContext;
use http::Method;


pub trait Filter {
    fn check(&self, req: &RequestContext) -> bool;
}

struct FnFilter<F: Fn(&RequestContext) -> bool>(F);

impl<F: Fn(&RequestContext) -> bool> Filter for FnFilter<F> {
    fn check(&self, req: &RequestContext) -> bool {
        (self.0)(req)
    }
}
pub fn fn_filter<F>(f: F) -> impl Filter
where
    F: Fn(&RequestContext) -> bool,
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
    pub fn or<F: Filter + 'static>(mut self, filter: F) -> Self {
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

/// compose filters with *AND* logic, if any inner filter success, the whole [`AllFilter`] will success
pub struct AllFilter {
    filters: Vec<Box<dyn Filter>>,
}

impl AllFilter {
    fn new() -> Self {
        Self { filters: vec![] }
    }
    pub fn or<F: Filter + 'static>(mut self, filter: F) -> Self {
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

pub fn get() -> MethodFilter {
    MethodFilter(Method::GET)
}

pub fn post() -> MethodFilter {
    MethodFilter(Method::POST)
}

pub struct MethodFilter(Method);

impl Filter for MethodFilter {
    fn check(&self, req: &RequestContext) -> bool {
        self.0.eq(req.method())
    }
}
