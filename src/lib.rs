extern crate core;

pub mod connection;
pub mod handler;
pub mod protocol;
pub mod codec;

//todo: need a own error type
pub type Error = anyhow::Error;
pub type Result<T> = anyhow::Result<T>;