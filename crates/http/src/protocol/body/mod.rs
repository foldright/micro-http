//! HTTP request body handling implementation.
//!
//! This module provides the core abstractions and implementations for handling HTTP request bodies
//! in an efficient streaming manner. The design focuses on:
//!
//! - Memory efficiency through streaming
//! - Protocol correctness
//! - Clean abstraction boundaries
//! - Concurrent processing capabilities
//!
//! # Architecture
//!
//! The body handling system consists of two main components:
//!
//! - [`ReqBody`]: The consumer side that implements `http_body::Body` trait
//! - [`ReqBodyState`]: Connection-side guard that owns the streaming state
//!
//! The connection hands a [`ReqBody`] to the request handler and keeps the
//! associated [`ReqBodyState`]. Once the handler finishes, the connection uses
//! the state to finish draining any unread body data and to reclaim ownership of
//! the underlying decoder. This design removes the previous channel-based
//! indirection and lets the handler poll the decoder directly.
//!
//! # Design Goals
//!
//! 1. **Memory Efficiency**
//!    - Stream body chunks instead of buffering entire payload
//!    - Implement backpressure to prevent overwhelming memory
//!
//! 2. **Protocol Correctness**
//!    - Ensure complete body consumption even if handler abandons reading
//!    - Maintain proper connection state for keep-alive support
//!
//! 3. **Graceful Cancellation**
//!    - Ensure complete body consumption even if the handler drops the body
//!      without reading it
//!    - Support cleanup in error cases without spawning helper tasks
//!
//! 4. **Clean Abstractions**
//!    - Hide channel complexity from consumers
//!    - Provide standard http_body::Body interface
//!
//! # Implementation Details
//!
mod req_body;

pub use req_body::ReqBody;
#[allow(unused_imports)]
pub(crate) use req_body::ReqBodyState;
