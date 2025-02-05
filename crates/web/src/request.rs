//! Request handling module that provides access to HTTP request information and path parameters.
//!
//! This module contains the core types for working with HTTP requests in the web framework:
//! - `RequestContext`: Provides access to request headers and path parameters
//! - `PathParams`: Handles URL path parameters extracted from request paths

use http::{HeaderMap, Method, Uri, Version};
use matchit::Params;
use micro_http::protocol::RequestHeader;

/// Represents the context of an HTTP request, providing access to both the request headers
/// and any path parameters extracted from the URL.
///
/// The lifetime parameters ensure that the request context does not outlive the server
/// or the request data it references.
pub struct RequestContext<'server: 'req, 'req> {
    request_header: &'req RequestHeader,
    path_params: &'req PathParams<'server, 'req>,
}

impl<'server, 'req> RequestContext<'server, 'req> {
    /// Creates a new RequestContext with the given request header and path parameters
    pub fn new(request_header: &'req RequestHeader, path_params: &'req PathParams<'server, 'req>) -> Self {
        Self { request_header, path_params }
    }

    /// Returns a reference to the underlying RequestHeader
    pub fn request_header(&self) -> &RequestHeader {
        self.request_header
    }

    /// Returns the HTTP method of the request
    pub fn method(&self) -> &Method {
        self.request_header.method()
    }

    /// Returns the URI of the request
    pub fn uri(&self) -> &Uri {
        self.request_header.uri()
    }

    /// Returns the HTTP version of the request
    pub fn version(&self) -> Version {
        self.request_header.version()
    }

    /// Returns the HTTP headers of the request
    pub fn headers(&self) -> &HeaderMap {
        self.request_header.headers()
    }

    /// Returns a reference to the path parameters extracted from the request URL
    pub fn path_params(&self) -> &PathParams {
        self.path_params
    }
}

/// Represents path parameters extracted from the URL path of an HTTP request.
///
/// Path parameters are named segments in the URL path that can be extracted and accessed
/// by name. For example, in the path "/users/{id}", "id" is a path parameter.
#[derive(Debug, Clone)]
pub struct PathParams<'server, 'req> {
    kind: PathParamsKind<'server, 'req>,
}

/// Internal enum to represent either empty parameters or actual parameters
#[derive(Debug, Clone)]
enum PathParamsKind<'server, 'req> {
    None,
    Params(Params<'server, 'req>),
}

impl<'server, 'req> PathParams<'server, 'req> {
    /// Creates a new PathParams instance from the given Params
    /// If the params are empty, returns an empty PathParams instance
    #[inline]
    fn new(params: Params<'server, 'req>) -> Self {
        if params.is_empty() {
            Self::empty()
        } else {
            Self { kind: PathParamsKind::Params(params) }
        }
    }

    /// Creates an empty PathParams instance with no parameters
    #[inline]
    pub fn empty() -> Self {
        Self { kind: PathParamsKind::None }
    }

    /// Returns true if there are no path parameters
    #[inline]
    pub fn is_empty(&self) -> bool {
        match &self.kind {
            PathParamsKind::None => true,
            PathParamsKind::Params(params) => params.is_empty(),
        }
    }

    /// Returns the number of path parameters
    #[inline]
    pub fn len(&self) -> usize {
        match &self.kind {
            PathParamsKind::None => 0,
            PathParamsKind::Params(params) => params.len(),
        }
    }

    /// Gets the value of a path parameter by its name
    /// Returns None if the parameter doesn't exist
    #[inline]
    pub fn get(&self, key: impl AsRef<str>) -> Option<&'req str> {
        match &self.kind {
            PathParamsKind::Params(params) => params.get(key),
            PathParamsKind::None => None,
        }
    }
}

// Implementation of From trait to convert from Params to PathParams
impl<'server, 'req> From<Params<'server, 'req>> for PathParams<'server, 'req> {
    fn from(params: Params<'server, 'req>) -> Self {
        PathParams::new(params)
    }
}
