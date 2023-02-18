extern crate core;

pub mod codec;
pub mod connection;
pub mod handler;
pub mod protocol;

pub(crate) use help::ensure;

mod help {

    macro_rules! ensure {
        ($predicate:expr, $error:expr) => {
            if !$predicate {
                return Err($error);
            }
        };
    }
    pub(crate) use ensure;
}
