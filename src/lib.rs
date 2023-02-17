extern crate core;

pub mod codec;
pub mod connection;
pub mod handler;
pub mod protocol;

#[macro_export]
macro_rules! ensure {
    ($predicate:expr, $error:expr) => {
        if !$predicate {
            return Err($error);
        }
    };
}
