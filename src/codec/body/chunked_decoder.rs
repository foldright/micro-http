use crate::codec::body::chunked_decoder::ChunkedState::*;
use crate::codec::body::payload_decoder::PayloadItem;
use bytes::{Buf, Bytes, BytesMut};
use std::io;
use std::io::ErrorKind;
use std::task::Poll;
use tokio_util::codec::Decoder;
use tracing::trace;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkedDecoder {
    state: ChunkedState,
    remaining_size: u64,
}

impl ChunkedDecoder {
    pub fn new() -> Self {
        Self { state: Size, remaining_size: 0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChunkedState {
    Size,
    SizeLws,
    Extension,
    SizeLf,
    Body,
    BodyCr,
    BodyLf,
    Trailer,
    TrailerLf,
    EndCr,
    EndLf,
    End,
}

impl Decoder for ChunkedDecoder {
    type Item = PayloadItem;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            if self.state == End {
                trace!("finished reading chunked data");
                return Ok(Some(PayloadItem::Eof));
            }

            if src.len() == 0 {
                // need more data
                return Ok(None);
            }

            let mut buf = None;

            self.state = match self.state.step(src, &mut self.remaining_size, &mut buf) {
                Poll::Pending => return Ok(None),
                Poll::Ready(Ok(new_state)) => new_state,
                Poll::Ready(Err(e)) => return Err(e),
            };

            if let Some(bytes) = buf {
                trace!(len = bytes.len(), "read chunked bytes");
                return Ok(Some(PayloadItem::Chunk(bytes)));
            }
        }
    }
}

macro_rules! try_next_byte {
    ($src:ident) => {{
        if $src.len() > 0 {
            $src.get_u8()
        } else {
            return Poll::Pending;
        }
    }};
}

impl ChunkedState {
    fn step(
        &self,
        src: &mut BytesMut,
        remaining_size: &mut u64,
        buf: &mut Option<Bytes>,
    ) -> Poll<Result<ChunkedState, io::Error>> {
        match self {
            Size => ChunkedState::read_size(src, remaining_size),
            SizeLws => ChunkedState::read_size_lws(src),
            Extension => ChunkedState::read_extension(src),
            SizeLf => ChunkedState::read_size_lf(src, remaining_size),
            Body => ChunkedState::read_body(src, remaining_size, buf),
            BodyCr => ChunkedState::read_body_cr(src),
            BodyLf => ChunkedState::read_body_lf(src),
            Trailer => ChunkedState::read_trailer(src),
            TrailerLf => ChunkedState::read_trailer_lf(src),
            EndCr => ChunkedState::read_end_cr(src),
            EndLf => ChunkedState::read_end_lf(src),
            End => Poll::Ready(Ok(End)),
        }
    }

    fn read_size(src: &mut BytesMut, size_per_chunk: &mut u64) -> Poll<Result<ChunkedState, io::Error>> {
        macro_rules! or_overflow {
            ($e:expr) => {
                match $e {
                    Some(val) => val,
                    None => {
                        return Poll::Ready(Err(io::Error::new(
                            ErrorKind::InvalidInput,
                            "invalid overflow chunked length",
                        )))
                    }
                }
            };
        }

        let radix = 16;
        match try_next_byte!(src) {
            b @ b'0'..=b'9' => {
                *size_per_chunk = or_overflow!(size_per_chunk.checked_mul(radix));
                *size_per_chunk = or_overflow!(size_per_chunk.checked_add((b - b'0') as u64));
            }

            b @ b'a'..=b'f' => {
                *size_per_chunk = or_overflow!(size_per_chunk.checked_mul(radix));
                *size_per_chunk = or_overflow!(size_per_chunk.checked_add((b + 10 - b'a') as u64));
            }
            b @ b'A'..=b'F' => {
                *size_per_chunk = or_overflow!(size_per_chunk.checked_mul(radix));
                *size_per_chunk = or_overflow!(size_per_chunk.checked_add((b + 10 - b'A') as u64));
            }
            b'\t' | b' ' => return Poll::Ready(Ok(SizeLws)),
            b';' => return Poll::Ready(Ok(Extension)),
            b'\r' => return Poll::Ready(Ok(SizeLf)),

            _ => {
                return Poll::Ready(Err(io::Error::new(
                    ErrorKind::InvalidInput,
                    "invalid chunk size line: Invalid Size",
                )))
            }
        }

        Poll::Ready(Ok(Size))
    }

    fn read_size_lws(src: &mut BytesMut) -> Poll<Result<ChunkedState, io::Error>> {
        match try_next_byte!(src) {
            // LWS can follow the chunk size, but no more digits can come
            b'\t' | b' ' => Poll::Ready(Ok(SizeLws)),
            b';' => Poll::Ready(Ok(Extension)),
            b'\r' => Poll::Ready(Ok(SizeLf)),
            _ => Poll::Ready(Err(io::Error::new(ErrorKind::InvalidInput, "invalid chunk size linear white space"))),
        }
    }

    fn read_extension(src: &mut BytesMut) -> Poll<Result<ChunkedState, io::Error>> {
        // We don't care about extensions really at all. Just ignore them.
        // They "end" at the next CRLF.
        //
        // However, some implementations may not check for the CR, so to save
        // them from themselves, we reject extensions containing plain LF as
        // well.
        match try_next_byte!(src) {
            b'\r' => Poll::Ready(Ok(SizeLf)),
            b'\n' => {
                Poll::Ready(Err(io::Error::new(ErrorKind::InvalidInput, "invalid chunk extension contains newline")))
            }
            _ => Poll::Ready(Ok(Extension)), // no supported extensions
        }
    }

    fn read_size_lf(src: &mut BytesMut, size_per_chunk: &mut u64) -> Poll<Result<ChunkedState, io::Error>> {
        match try_next_byte!(src) {
            b'\n' => {
                if *size_per_chunk == 0 {
                    Poll::Ready(Ok(EndCr))
                } else {
                    Poll::Ready(Ok(Body))
                }
            }

            _ => Poll::Ready(Err(io::Error::new(ErrorKind::InvalidInput, "invalid chunk size LF"))),
        }
    }

    fn read_body(
        src: &mut BytesMut,
        size_per_chunk: &mut u64,
        buf: &mut Option<Bytes>,
    ) -> Poll<Result<ChunkedState, io::Error>> {
        if src.is_empty() {
            return Poll::Ready(Ok(Body));
        }

        if *size_per_chunk == 0 {
            return Poll::Ready(Ok(BodyCr));
        }

        // cap remaining bytes at the max capacity of usize
        let remaining = match *size_per_chunk {
            r if r > usize::MAX as u64 => usize::MAX,
            r => r as usize,
        };

        let read_size = std::cmp::min(remaining, src.len());

        *size_per_chunk -= read_size as u64;
        let bytes = src.split_to(read_size).freeze();
        *buf = Some(bytes);

        if *size_per_chunk > 0 {
            Poll::Ready(Ok(Body))
        } else {
            Poll::Ready(Ok(BodyCr))
        }
    }

    fn read_body_cr(src: &mut BytesMut) -> Poll<Result<ChunkedState, io::Error>> {
        match try_next_byte!(src) {
            b'\r' => Poll::Ready(Ok(BodyLf)),
            _ => Poll::Ready(Err(io::Error::new(ErrorKind::InvalidInput, "invalid chunk body CR"))),
        }
    }
    fn read_body_lf(src: &mut BytesMut) -> Poll<Result<ChunkedState, io::Error>> {
        match try_next_byte!(src) {
            b'\n' => Poll::Ready(Ok(Size)),
            _ => Poll::Ready(Err(io::Error::new(ErrorKind::InvalidInput, "invalid chunk body LF"))),
        }
    }

    fn read_trailer(src: &mut BytesMut) -> Poll<Result<ChunkedState, io::Error>> {
        match try_next_byte!(src) {
            b'\r' => Poll::Ready(Ok(TrailerLf)),
            _ => Poll::Ready(Ok(Trailer)),
        }
    }
    fn read_trailer_lf(src: &mut BytesMut) -> Poll<Result<ChunkedState, io::Error>> {
        match try_next_byte!(src) {
            b'\n' => Poll::Ready(Ok(EndCr)),
            _ => Poll::Ready(Err(io::Error::new(ErrorKind::InvalidInput, "invalid trailer end LF"))),
        }
    }

    fn read_end_cr(src: &mut BytesMut) -> Poll<Result<ChunkedState, io::Error>> {
        match try_next_byte!(src) {
            b'\r' => Poll::Ready(Ok(EndLf)),
            _ => Poll::Ready(Ok(Trailer)),
        }
    }
    fn read_end_lf(src: &mut BytesMut) -> Poll<Result<ChunkedState, io::Error>> {
        match try_next_byte!(src) {
            b'\n' => Poll::Ready(Ok(End)),
            _ => Poll::Ready(Err(io::Error::new(ErrorKind::InvalidInput, "invalid chunk end LF"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::codec::ChunkedDecoder;
    use bytes::BytesMut;
    use tokio_util::codec::Decoder;

    #[test]
    fn test_basic() {
        let mut buffer: BytesMut = BytesMut::from(&b"10\r\n1234567890abcdef\r\n0\r\n\r\n"[..]);
        let mut decoder = ChunkedDecoder::new();
        {
            let result = decoder.decode(&mut buffer);
            assert!(result.is_ok());

            let option = result.unwrap();
            assert!(option.is_some());

            let item = option.unwrap();
            assert!(item.is_chunk());
            assert_eq!(item.bytes().unwrap().len(), 16);

            let str = std::str::from_utf8(&item.bytes().unwrap()[..]).unwrap();

            assert_eq!(str, "1234567890abcdef");
        }

        {
            let result = decoder.decode(&mut buffer);
            assert!(result.is_ok());

            let option = result.unwrap();
            assert!(option.is_some());

            assert!(option.unwrap().is_eof());
        }
    }
}
