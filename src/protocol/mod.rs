pub mod body;
mod message;
mod request;
mod response;

pub use request::RequestHeader;
pub use response::ResponseHead;
use snafu::Snafu;

use crate::codec::{DecodeError, EncoderError};
pub use message::Message;
pub use message::PayloadItem;
pub use message::PayloadSize;

#[derive(Debug, Snafu)]
pub enum HttpError {
    #[snafu(display("request error: {source}"), context(false))]
    RequestError {
        source: DecodeError,
    },

    #[snafu(display("response error: {source}"), context(false))]
    ResponseError {
        source: EncoderError,
    },
}
