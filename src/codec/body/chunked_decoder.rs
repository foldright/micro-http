use anyhow::anyhow;
use bytes::{Buf, Bytes, BytesMut};
use tokio_util::codec::Decoder;
use crate::codec::body::BodyData;
use crate::codec::body::chunked_decoder::ChunkedState::*;


pub struct ChunkedDecoder {
    state: ChunkedState,
    remaining_size: u64,
}

impl ChunkedDecoder {
    pub fn new() -> Self {
        Self {
            state: Size,
            remaining_size: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
    type Item = BodyData;
    type Error = crate::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            if self.state == End {
                return Ok(Some(BodyData::Finished));
            }

            if src.len() == 0 {
                return Ok(None);
            }

            match self.state.step(src, &mut self.remaining_size) {
                Ok((new_state, None)) => {
                    self.state = new_state;
                    continue;
                }
                Ok((new_state, Some(bytes))) => {
                    self.state = new_state;
                    return Ok(Some(BodyData::Bytes(bytes)));
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }
}

impl ChunkedState {
    fn step(&self, src: &mut BytesMut, remaining_size: &mut u64) -> crate::Result<(ChunkedState, Option<Bytes>)> {
        match self {
            Size => ChunkedState::read_size(src, remaining_size),
            SizeLws => ChunkedState::read_size_lws(src, remaining_size),
            Extension => ChunkedState::read_extension(src, remaining_size),
            SizeLf => ChunkedState::read_size_lf(src, remaining_size),
            Body => ChunkedState::read_body(src, remaining_size),
            BodyCr => ChunkedState::read_body_cr(src, remaining_size),
            BodyLf => ChunkedState::read_body_lf(src, remaining_size),
            Trailer => ChunkedState::read_trailer(src, remaining_size),
            TrailerLf => ChunkedState::read_trailer_lf(src, remaining_size),
            EndCr => ChunkedState::read_end_cr(src, remaining_size),
            EndLf => ChunkedState::read_end_lf(src, remaining_size),
            End => Ok((End, None)),
        }
    }

    fn read_size(src: &mut BytesMut, size_per_chunk: &mut u64) -> crate::Result<(ChunkedState, Option<Bytes>)> {
        macro_rules! or_overflow {
            ($e:expr) => (
                match $e {
                    Some(val) => val,
                    None => return Err(anyhow!("invalid chunk size: overflow")),
                }
            )
        }

        let radix = 16;
        match src.get_u8() {
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
            b'\t' | b' ' => return Ok((SizeLws, None)),
            b';' => return Ok((Extension, None)),
            b'\r' => return Ok((SizeLf, None)),

            _ => return Err(anyhow!("invalid chunk size line: Invalid Size"))
        }

        Ok((Size, None))
    }

    fn read_size_lws(src: &mut BytesMut, _size_per_chunk: &mut u64) -> crate::Result<(ChunkedState, Option<Bytes>)> {
        match src.get_u8() {
            // LWS can follow the chunk size, but no more digits can come
            b'\t' | b' ' => Ok((ChunkedState::SizeLws, None)),
            b';' => Ok((ChunkedState::Extension, None)),
            b'\r' => Ok((ChunkedState::SizeLf, None)),
            _ => Err(anyhow!("invalid chunk size linear white space")),
        }
    }

    fn read_extension(src: &mut BytesMut, _size_per_chunk: &mut u64) -> crate::Result<(ChunkedState, Option<Bytes>)> {
        // We don't care about extensions really at all. Just ignore them.
        // They "end" at the next CRLF.
        //
        // However, some implementations may not check for the CR, so to save
        // them from themselves, we reject extensions containing plain LF as
        // well.
        match src.get_u8() {
            b'\r' => Ok((ChunkedState::SizeLf, None)),
            b'\n' => Err(anyhow!("invalid chunk extension contains newline")),
            _ => Ok((ChunkedState::Extension, None)), // no supported extensions
        }
    }

    fn read_size_lf(src: &mut BytesMut, size_per_chunk: &mut u64) -> crate::Result<(ChunkedState, Option<Bytes>)> {
        match src.get_u8() {
            b'\n' => {
                if *size_per_chunk == 0 {
                    Ok((EndCr, None))
                } else {
                    Ok((Body, None))
                }
            }
            _ => Err(anyhow!("invalid chunk size LF")),
        }
    }

    fn read_body(src: &mut BytesMut, size_per_chunk: &mut u64) -> crate::Result<(ChunkedState, Option<Bytes>)> {
        if src.is_empty() {
            return Ok((Body, None));
        }

        if *size_per_chunk == 0 {
            return Ok((BodyCr, None));
        }

        // cap remaining bytes at the max capacity of usize
        let remaining = match *size_per_chunk {
            r if r > usize::MAX as u64 => usize::MAX,
            r => r as usize,
        };

        let read_size = std::cmp::min(remaining, src.len());

        *size_per_chunk -= read_size as u64;
        let bytes = src.split_to(read_size).freeze();

        if *size_per_chunk > 0 {
            Ok((Body, Some(bytes)))
        } else {
            Ok((BodyCr, Some(bytes)))
        }
    }

    fn read_body_cr(src: &mut BytesMut, _size_per_chunk: &mut u64) -> crate::Result<(ChunkedState, Option<Bytes>)> {
        match src.get_u8() {
            b'\r' => Ok((BodyLf, None)),
            _ => Err(anyhow!("invalid chunk body CR")),
        }
    }
    fn read_body_lf(src: &mut BytesMut, _size_per_chunk: &mut u64) -> crate::Result<(ChunkedState, Option<Bytes>)> {
        match src.get_u8() {
            b'\n' => Ok((Size, None)),
            _ => Err(anyhow!("invalid chunk body LF")),
        }
    }

    fn read_trailer(src: &mut BytesMut, _size_per_chunk: &mut u64) -> crate::Result<(ChunkedState, Option<Bytes>)> {
        match src.get_u8() {
            b'\r' => Ok((TrailerLf, None)),
            _ => Ok((Trailer, None)),
        }
    }
    fn read_trailer_lf(src: &mut BytesMut, _size_per_chunk: &mut u64) -> crate::Result<(ChunkedState, Option<Bytes>)> {
        match src.get_u8() {
            b'\n' => Ok((EndCr, None)),
            _ => Err(anyhow!("invalid trailer end LF")),
        }
    }

    fn read_end_cr(src: &mut BytesMut, _size_per_chunk: &mut u64) -> crate::Result<(ChunkedState, Option<Bytes>)> {
        match src.get_u8() {
            b'\r' => Ok((EndLf, None)),
            _ => Ok((Trailer, None)),
        }
    }
    fn read_end_lf(src: &mut BytesMut, _size_per_chunk: &mut u64) -> crate::Result<(ChunkedState, Option<Bytes>)> {
        match src.get_u8() {
            b'\n' => Ok((End, None)),
            _ => Err(anyhow!("invalid chunk end LF")),
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;
    use tokio_util::codec::Decoder;
    use crate::codec::ChunkedDecoder;

    #[test]
    fn test_basic() {
        let mut buffer: BytesMut = BytesMut::from(&b"10\r\n1234567890abcdef\r\n0\r\n\r\n"[..]);
        let mut decoder = ChunkedDecoder::new();
        {
            let result = decoder.decode(&mut buffer);
            assert!(result.is_ok());

            let option = result.unwrap();
            assert!(option.is_some());

            let bytes = option.unwrap().into_bytes().unwrap();
            assert_eq!(bytes.len(), 16);

            let str = std::str::from_utf8(&bytes[..]).unwrap();

            assert_eq!(str, "1234567890abcdef");
        }

        {
            let result = decoder.decode(&mut buffer);
            assert!(result.is_ok());

            let option = result.unwrap();
            assert!(option.is_some());

            assert!(option.unwrap().is_finished());
        }
    }
}