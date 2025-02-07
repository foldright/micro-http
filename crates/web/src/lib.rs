//! High-level web framework built on top of micro-http.
//!
//! This crate provides an ergonomic web framework that simplifies building HTTP services
//! by offering high-level abstractions while leveraging the efficient HTTP implementation
//! from the micro-http crate.
//!
//! # Core Features
//!
//! - **Ergonomic Request Handling**
//!   - Async function handlers with automatic type conversion
//!   - Flexible routing with path parameters
//!   - Built-in support for common data formats (JSON, form data, etc.)
//!
//! - **Middleware System**
//!   - Composable request/response transformations
//!   - Built-in middleware for common tasks (compression, date headers, etc.)
//!   - Easy to implement custom middleware
//!
//! - **Type-Safe Extractors**
//!   - Automatic request data extraction into Rust types
//!   - Support for headers, query parameters, and request bodies
//!   - Custom extractor implementation possible
//!
//! - **Flexible Response Types**
//!   - Automatic conversion of Rust types to HTTP responses
//!   - Streaming response support
//!   - Content negotiation and compression
//!
//! # Architecture
//!
//! The framework is organized into several key modules:
//!
//! - **Core Types** ([`handler`], [`request`], [`responder`])
//!   - Request handling traits and implementations
//!   - Context types for accessing request data
//!   - Response generation utilities
//!
//! - **Routing** ([`router`])
//!   - URL pattern matching
//!   - HTTP method filtering
//!   - Route parameter extraction
//!
//! - **Data Extraction** ([`extract`])
//!   - Query string parsing
//!   - Form data handling
//!   - JSON serialization/deserialization
//!
//! - **Request Filtering** ([`filter`])
//!   - Header-based filtering
//!   - Method matching
//!   - Custom filter implementation
//!
//! - **Middleware** ([`wrapper`])
//!   - Response transformation
//!   - Cross-cutting concerns
//!   - Built-in middleware components
//!
//! # Example
//!
//! ```no_run
//! use micro_web::{
//!     router::{get, Router},
//!     handler_fn,
//!     Server,
//! };
//!
//! // Define a simple handler
//! async fn hello_world() -> &'static str {
//!     "Hello, World!"
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create a router
//!     let router = Router::builder()
//!         .route("/", get(handler_fn(hello_world)))
//!         .build();
//!
//!     // Start the server
//!     Server::builder()
//!         .router(router)
//!         .bind("127.0.0.1:3000")
//!         .build()
//!         .unwrap()
//!         .start()
//!         .await;
//! }
//! ```
//!
//! # Relationship with micro-http
//!
//! This framework builds upon the low-level HTTP implementation provided by micro-http:
//!
//! - micro-http handles the raw TCP connections and HTTP protocol details
//! - This crate provides the high-level abstractions for building web services
//! - The integration is seamless while maintaining performance
//!
//! See the [micro-http documentation](micro_http) for more details about the underlying implementation.

// Internal modules
mod body;
mod fn_trait;
mod handler;
mod request;
mod responder;
mod server;

// Public modules
pub mod extract;
pub mod router;
pub mod decorator;
pub mod encoding;
pub mod date;

// Public re-exports
pub use body::OptionReqBody;
pub use body::ResponseBody;
pub use fn_trait::FnTrait;
pub use handler::handler_fn;
pub use handler::FnHandler;
pub use request::PathParams;
pub use request::RequestContext;
pub use server::Server;
