use crate::protocol::{PayloadSize, ResponseHead};

use bytes::{BufMut, BytesMut};

use http::{header, Version};
use std::io;
use std::io::ErrorKind;
use tokio_util::codec::Encoder;
use tracing::error;

pub struct HeaderEncoder;

impl Encoder<(ResponseHead, PayloadSize)> for HeaderEncoder {
    type Error = io::Error;

    fn encode(&mut self, item: (ResponseHead, PayloadSize), dst: &mut BytesMut) -> Result<(), Self::Error> {
        let (mut header, payload_size) = item;

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
                return Err(io::Error::from(ErrorKind::Unsupported));
            }
        }

        match payload_size {
            PayloadSize::Length(_) => {}
            PayloadSize::Chunked => match header.headers_mut().get_mut(header::CONTENT_LENGTH) {
                Some(value) => *value = 0.into(),
                None => {
                    header.headers_mut().insert(header::CONTENT_LENGTH, "chunked".parse().unwrap());
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
            dst.put_slice(header_name.as_str().as_bytes());
            dst.put_slice(b": ");
            dst.put_slice(header_value.as_bytes());
            dst.put_slice(b"\r\n");
        }
        dst.put_slice(b"\r\n");
        Ok(())
    }
}
