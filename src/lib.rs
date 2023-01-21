pub mod server;
pub mod connection;
pub mod protocol;

//todo: need a own error type
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;