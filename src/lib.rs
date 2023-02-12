extern crate core;

pub mod codec;
pub mod connection;
pub mod handler;
pub mod protocol;

//todo: need a own error type
pub type Error = anyhow::Error;
pub type Result<T> = anyhow::Result<T>;
