//! Core HTTP protocol abstractions and implementations.
//! 
//! This module provides the fundamental building blocks for HTTP protocol handling,
//! including request/response processing, body streaming, and error handling.
//! The design focuses on providing clean abstractions while maintaining high performance
//! and memory efficiency.
//!
//! # Architecture
//!
//! The protocol module is organized into several key components:
//!
//! - **Message Handling** ([`message`]): Core message types and payload processing
//!   - [`Message`]: Represents either headers or payload chunks
//!   - [`PayloadItem`]: Handles individual payload chunks and EOF
//!   - [`PayloadSize`]: Tracks payload size information
//!
//! - **Request Processing** ([`request`]): Request header handling
//!   - [`RequestHeader`]: Wraps HTTP request headers with additional functionality
//!
//! - **Response Processing** ([`response`]): Response header handling
//!   - [`ResponseHead`]: Type alias for response headers before body attachment
//!
//! - **Body Streaming** ([`body`]): Efficient body handling implementation
//!   - [`ReqBody`]: Consumer side implementing `http_body::Body`
//!   - [`ReqBodySender`]: Producer side for streaming body chunks
//!
//! - **Error Handling** ([`error`]): Comprehensive error types
//!   - [`HttpError`]: Top-level error type
//!   - [`ParseError`]: Request parsing errors
//!   - [`SendError`]: Response sending errors
//!
//! # Design Goals
//!
//! 1. **Memory Efficiency**
//!    - Stream request/response bodies instead of buffering
//!    - Implement proper backpressure mechanisms
//!
//! 2. **Clean Abstractions**
//!    - Provide intuitive interfaces for protocol handling
//!    - Hide implementation complexity from consumers
//!
//! 3. **Protocol Correctness**
//!    - Ensure proper HTTP protocol compliance
//!    - Handle edge cases and error conditions gracefully
//!
//! 4. **Performance**
//!    - Minimize allocations and copies
//!    - Support concurrent processing where beneficial
//!
//! # Example Usage
//!
//! The protocol module is typically used through the connection layer rather than
//! directly. However, understanding its components is crucial for implementing
//! custom handlers or extending functionality.
//!
//! See individual module documentation for specific usage examples.

mod message;
pub use message::Message;
pub use message::PayloadItem;
pub use message::PayloadSize;

mod request;
pub use request::RequestHeader;

mod response;
pub use response::ResponseHead;

mod error;
pub use error::HttpError;
pub use error::ParseError;
pub use error::SendError;

pub mod body;
