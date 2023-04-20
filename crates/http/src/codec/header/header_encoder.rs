use crate::protocol::{PayloadSize, ResponseHead, SendError};

use bytes::{BufMut, BytesMut};

use http::{header, Version};
use std::io;
use std::io::ErrorKind;
use tokio_util::codec::Encoder;
use tracing::error;

const INIT_HEADER_SIZE: usize = 4 * 1024;

pub struct HeaderEncoder;

impl Encoder<(ResponseHead, PayloadSize)> for HeaderEncoder {
    type Error = SendError;

    fn encode(&mut self, item: (ResponseHead, PayloadSize), dst: &mut BytesMut) -> Result<(), Self::Error> {
        let (mut header, payload_size) = item;

        dst.reserve(INIT_HEADER_SIZE);
        match header.version() {
            Version::HTTP_11 => {
                dst.put_slice(b"HTTP/1.1 ");
                dst.put_slice(header.status().as_str().as_bytes());
                dst.put_slice(b" ");
                dst.put_slice(header.status().canonical_reason().unwrap().as_bytes());
                dst.put_slice(b"\r\n");
            }
            v => {
                error!(http_version = ?v, "unsupported http version");
                return Err(io::Error::from(ErrorKind::Unsupported).into());
            }
        }

        match payload_size {
            PayloadSize::Length(n) => match header.headers_mut().get_mut(header::CONTENT_LENGTH) {
                Some(value) => *value = n.into(),
                None => {
                    header.headers_mut().insert(header::CONTENT_LENGTH, n.into());
                }
            },
            PayloadSize::Chunked => match header.headers_mut().get_mut(header::TRANSFER_ENCODING) {
                Some(value) => *value = "chunked".parse().unwrap(),
                None => {
                    header.headers_mut().insert(header::TRANSFER_ENCODING, "chunked".parse().unwrap());
                }
            },
            PayloadSize::Empty => match header.headers_mut().get_mut(header::CONTENT_LENGTH) {
                Some(value) => *value = 0.into(),
                None => {
                    header.headers_mut().insert(header::CONTENT_LENGTH, 0.into());
                }
            },
        }

        for (header_name, header_value) in header.headers().iter() {
            dst.put_slice(header_name.as_ref());
            dst.put_slice(b": ");
            dst.put_slice(header_value.as_ref());
            dst.put_slice(b"\r\n");
        }
        dst.put_slice(b"\r\n");
        Ok(())
    }
}
