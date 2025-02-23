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
//! - [`ReqBodySender`]: The producer side that reads from the raw payload stream
//!
//! These components communicate through channels to enable concurrent processing while
//! maintaining backpressure.
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
//! 3. **Concurrent Processing**
//!    - Allow request handling to proceed while body streams
//!    - Support cancellation and cleanup in error cases
//!
//! 4. **Clean Abstractions**
//!    - Hide channel complexity from consumers
//!    - Provide standard http_body::Body interface
//!
//! # Implementation Details
//!
//! The body handling implementation uses:
//!
//! - MPSC channel for signaling between consumer and producer
//! - Oneshot channels for individual chunk transfers
//! - EOF tracking to ensure complete body consumption
//! - Automatic cleanup of unread data
//!
//! See individual component documentation for more details.

//mod req_body_2;
mod body_channel;
mod req_body;

//pub use req_body_2::ReqBody2;
pub use req_body::ReqBody;
//pub use req_body_2::ReqBodySender;

