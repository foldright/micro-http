//! Utility macros and functions for the HTTP crate.
//! 
//! This module provides helper macros and functions that are used internally
//! by the HTTP crate implementation.

/// A macro for early returns with an error if a condition is not met.
/// 
/// This is similar to the `assert!` macro, but returns an error instead of panicking.
/// It's useful for validation checks where you want to return early with an error
/// if some condition is not satisfied.
/// 
/// # Arguments
/// 
/// * `$predicate` - A boolean expression that should evaluate to true
/// * `$error` - The error value to return if the predicate is false
/// 
/// # Example
/// 
/// ```
/// ensure!(headers.len() < MAX_HEADERS, ParseError::TooManyHeaders);
/// ```
macro_rules! ensure {
    ($predicate:expr, $error:expr) => {
        if !$predicate {
            return Err($error);
        }
    };
}

pub(crate) use ensure;
