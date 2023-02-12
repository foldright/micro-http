use anyhow::anyhow;
use bytes::{BufMut, BytesMut};
use http::response::Parts;
use http::Version;
use tokio_util::codec::Encoder;

pub struct HeaderEncoder;

impl Encoder<Parts> for HeaderEncoder {
    type Error = crate::Error;

    fn encode(&mut self, header: Parts, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match header.version {
            Version::HTTP_11 => {
                dst.put_slice(b"HTTP/1.1 ");
                dst.put_slice(header.status.as_str().as_bytes());
                dst.put_slice(b" ");
                dst.put_slice(header.status.canonical_reason().unwrap().as_bytes());
                dst.put_slice(b"\r\n");
            }
            v => return Err(anyhow!("not support http version {:?}", v)),
        }

        for (header_name, header_value) in header.headers.iter() {
            dst.put_slice(header_name.as_str().as_bytes());
            dst.put_slice(b": ");
            dst.put_slice(header_value.as_bytes());
            dst.put_slice(b"\r\n");
        }
        dst.put_slice(b"\r\n");
        Ok(())
    }
}
