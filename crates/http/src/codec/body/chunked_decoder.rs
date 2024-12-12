//! Decoder implementation for HTTP chunked transfer encoding.
//! 
//! This module provides functionality to decode HTTP messages that use chunked transfer encoding
//! as specified in [RFC 7230 Section 4.1](https://tools.ietf.org/html/rfc7230#section-4.1).
//!
//! The chunked encoding allows the sender to transmit message data in a series of chunks,
//! indicating the size of each chunk before its data.

use crate::protocol::{ParseError, PayloadItem};
use bytes::{Buf, Bytes, BytesMut};
use std::io;
use std::io::ErrorKind;
use std::task::Poll;
use tokio_util::codec::Decoder;
use tracing::trace;
use ChunkedState::*;

/// A decoder for handling HTTP chunked transfer encoding.
///
/// The decoder processes incoming bytes according to the chunked format:
/// - Each chunk starts with its size in hexadecimal
/// - Followed by optional extensions and CRLF
/// - Then the chunk data and CRLF
/// - A zero-sized chunk indicates the end of the message
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkedDecoder {
    state: ChunkedState,
    remaining_size: u64,
}

impl ChunkedDecoder {
    /// Creates a new ChunkedDecoder instance.
    ///
    /// The decoder starts in the Size state, ready to read the size of the first chunk.
    pub fn new() -> Self {
        Self { state: Size, remaining_size: 0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChunkedState {
    /// Read the chunk size in hex
    Size,
    /// Handle whitespace after size 
    SizeLws,
    /// Skip chunk extensions
    Extension,
    /// Read LF after chunk size
    SizeLf,
    /// Read chunk data
    Body,
    /// Read CR after chunk data
    BodyCr,
    /// Read LF after chunk data
    BodyLf,
    /// Read optional trailer fields
    Trailer,
    /// Read LF after trailer
    TrailerLf,
    /// Read final CR
    EndCr,
    /// Read final LF
    EndLf,
    /// Final state after reading last chunk
    End,
}

impl Decoder for ChunkedDecoder {
    type Item = PayloadItem;
    type Error = ParseError;

    /// Decodes chunked transfer encoded data from the input buffer.
    ///
    /// # Returns
    /// - `Ok(Some(PayloadItem::Chunk(bytes)))` when a chunk is successfully decoded
    /// - `Ok(Some(PayloadItem::Eof))` when the final chunk is processed
    /// - `Ok(None)` when more data is needed
    /// - `Err(ParseError)` if the chunked encoding is invalid
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            if self.state == End {
                trace!("finished reading chunked data");
                return Ok(Some(PayloadItem::Eof));
            }

            if src.is_empty() {
                // need more data
                return Ok(None);
            }

            let mut buf = None;

            self.state = match self.state.step(src, &mut self.remaining_size, &mut buf) {
                Poll::Pending => return Ok(None),
                Poll::Ready(Ok(new_state)) => new_state,
                Poll::Ready(Err(e)) => return Err(ParseError::io(e)),
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
    
    /// Processes the next step in the chunked decoding state machine.
    ///
    /// Takes the current buffer of bytes and updates internal state based on the 
    /// chunked transfer encoding format rules.
    ///
    /// # Arguments
    /// * `src` - Source buffer containing the chunked data
    /// * `remaining_size` - Tracks remaining bytes in current chunk
    /// * `buf` - Buffer to store decoded chunk data
    ///
    /// # Returns
    /// The next state in the decoding process or an error if invalid encoding is detected
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

    /// Reads and parses the chunk size in hexadecimal format.
    ///
    /// The size is read digit by digit until a delimiter is encountered.
    /// Supports both uppercase and lowercase hex digits.
    ///
    /// # State Transitions
    /// - On hex digit (0-9, a-f, A-F): Stay in Size state to read more digits
    /// - On whitespace (tab/space): Transition to SizeLws state
    /// - On semicolon: Transition to Extension state to handle chunk extensions
    /// - On CR: Transition to SizeLf state to finish size line
    /// - On invalid character: Return error
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

    /// Processes linear whitespace (LWS) after the chunk size.
    ///
    /// According to the HTTP chunked encoding specification:
    /// - Only tabs and spaces are allowed as LWS after the chunk size
    /// - Extensions can follow LWS starting with a semicolon  
    /// - A CR indicates the end of the size line
    /// - Any other character is invalid
    ///
    /// State transitions:
    /// - On tab/space: Stay in SizeLws state to handle more whitespace
    /// - On semicolon: Move to Extension state to process chunk extensions
    /// - On CR: Move to SizeLf state to finish size line
    /// - On invalid char: Return error
    fn read_size_lws(src: &mut BytesMut) -> Poll<Result<ChunkedState, io::Error>> {
        match try_next_byte!(src) {
            // LWS can follow the chunk size, but no more digits can come
            b'\t' | b' ' => Poll::Ready(Ok(SizeLws)),
            b';' => Poll::Ready(Ok(Extension)), 
            b'\r' => Poll::Ready(Ok(SizeLf)),
            _ => Poll::Ready(Err(io::Error::new(ErrorKind::InvalidInput, "invalid chunk size linear white space"))),
        }
    }

    /// Processes chunk extensions in the chunked encoding format.
    ///
    /// According to the HTTP specification, chunks may have optional extensions
    /// after the chunk size. This implementation ignores extensions but validates
    /// their format:
    /// - Extensions end at CRLF
    /// - Plain LF is not allowed in extensions
    /// - Any other bytes are allowed and ignored
    ///
    /// # State Transitions
    /// - On CR: Move to SizeLf state to finish extension line
    /// - On LF: Return error as extensions must end with CRLF
    /// - On any other byte: Stay in Extension state
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

    /// Validates the LF byte after the chunk size line.
    ///
    /// After reading the chunk size and optional extensions, this function
    /// expects an LF byte to complete the size line. It also checks if this
    /// is the last chunk (size = 0).
    ///
    /// # State Transitions
    /// - On LF with size 0: Move to EndCr state for final CRLF
    /// - On LF with size > 0: Move to Body state to read chunk data
    /// - On any other byte: Return error
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

    /// Reads the actual chunk data bytes.
    ///
    /// This function reads up to size_per_chunk bytes from the input buffer.
    /// It handles cases where:
    /// - The input buffer is empty
    /// - The chunk size is 0
    /// - The chunk size exceeds usize::MAX
    ///
    /// # State Transitions
    /// - On empty input: Stay in Body state
    /// - On size = 0: Move to BodyCr state
    /// - After reading data with remaining size > 0: Stay in Body state
    /// - After reading data with remaining size = 0: Move to BodyCr state
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

    /// Validates the CR byte after chunk data.
    ///
    /// After reading all bytes in a chunk, this function expects a CR byte
    /// as part of the chunk's terminating CRLF.
    ///
    /// # State Transitions
    /// - On CR: Move to BodyLf state
    /// - On any other byte: Return error
    fn read_body_cr(src: &mut BytesMut) -> Poll<Result<ChunkedState, io::Error>> {
        match try_next_byte!(src) {
            b'\r' => Poll::Ready(Ok(BodyLf)),
            _ => Poll::Ready(Err(io::Error::new(ErrorKind::InvalidInput, "invalid chunk body CR"))),
        }
    }

    /// Validates the LF byte after chunk data.
    ///
    /// After the CR byte, this function expects an LF byte to complete
    /// the chunk's terminating CRLF.
    ///
    /// # State Transitions
    /// - On LF: Move back to Size state for next chunk
    /// - On any other byte: Return error
    fn read_body_lf(src: &mut BytesMut) -> Poll<Result<ChunkedState, io::Error>> {
        match try_next_byte!(src) {
            b'\n' => Poll::Ready(Ok(Size)),
            _ => Poll::Ready(Err(io::Error::new(ErrorKind::InvalidInput, "invalid chunk body LF"))),
        }
    }

    /// Processes optional trailer fields after the last chunk.
    ///
    /// The chunked encoding format allows for trailer fields after the
    /// zero-length chunk. This implementation reads but ignores them.
    ///
    /// # State Transitions
    /// - On CR: Move to TrailerLf state
    /// - On any other byte: Stay in Trailer state
    fn read_trailer(src: &mut BytesMut) -> Poll<Result<ChunkedState, io::Error>> {
        match try_next_byte!(src) {
            b'\r' => Poll::Ready(Ok(TrailerLf)),
            _ => Poll::Ready(Ok(Trailer)),
        }
    }

    /// Validates the LF byte after a trailer field.
    ///
    /// After a trailer field's CR, this function expects an LF byte.
    ///
    /// # State Transitions
    /// - On LF: Move to EndCr state
    /// - On any other byte: Return error
    fn read_trailer_lf(src: &mut BytesMut) -> Poll<Result<ChunkedState, io::Error>> {
        match try_next_byte!(src) {
            b'\n' => Poll::Ready(Ok(EndCr)),
            _ => Poll::Ready(Err(io::Error::new(ErrorKind::InvalidInput, "invalid trailer end LF"))),
        }
    }

    /// Validates the final CR byte of the chunked message.
    ///
    /// After all chunks and trailers, this function expects a CR byte
    /// as part of the terminating CRLF.
    ///
    /// # State Transitions
    /// - On CR: Move to EndLf state
    /// - On any other byte: Move to Trailer state to handle as trailer field
    fn read_end_cr(src: &mut BytesMut) -> Poll<Result<ChunkedState, io::Error>> {
        match try_next_byte!(src) {
            b'\r' => Poll::Ready(Ok(EndLf)),
            _ => Poll::Ready(Ok(Trailer)),
        }
    }

    /// Validates the final LF byte of the chunked message.
    ///
    /// After the final CR byte, this function expects an LF byte to
    /// complete the chunked message.
    ///
    /// # State Transitions
    /// - On LF: Move to End state
    /// - On any other byte: Return error
    fn read_end_lf(src: &mut BytesMut) -> Poll<Result<ChunkedState, io::Error>> {
        match try_next_byte!(src) {
            b'\n' => Poll::Ready(Ok(End)),
            _ => Poll::Ready(Err(io::Error::new(ErrorKind::InvalidInput, "invalid chunk end LF"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            assert_eq!(item.as_bytes().unwrap().len(), 16);

            let str = std::str::from_utf8(&item.as_bytes().unwrap()[..]).unwrap();

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

    #[test]
    fn test_multiple_chunks() {
        let mut buffer: BytesMut = BytesMut::from(
            &b"5\r\nhello\r\n7\r\n, world\r\n0\r\n\r\n"[..]
        );
        let mut decoder = ChunkedDecoder::new();
        
        // First chunk
        let chunk = decoder.decode(&mut buffer).unwrap().unwrap();
        assert_eq!(chunk.as_bytes().unwrap(), &Bytes::copy_from_slice(b"hello"));
        
        // Second chunk
        let chunk = decoder.decode(&mut buffer).unwrap().unwrap();
        assert_eq!(chunk.as_bytes().unwrap(), &Bytes::copy_from_slice(b", world"));
        
        // EOF
        let eof = decoder.decode(&mut buffer).unwrap().unwrap();
        assert!(eof.is_eof());
    }

    #[test]
    fn test_chunks_with_extensions() {
        let mut buffer: BytesMut = BytesMut::from(
            &b"5;chunk-ext=value\r\nhello\r\n0\r\n\r\n"[..]
        );
        let mut decoder = ChunkedDecoder::new();
        
        let chunk = decoder.decode(&mut buffer).unwrap().unwrap();
        assert_eq!(chunk.as_bytes().unwrap(), &Bytes::copy_from_slice(b"hello"));
        
        let eof = decoder.decode(&mut buffer).unwrap().unwrap();
        assert!(eof.is_eof());
    }

    #[test]
    fn test_chunks_with_trailers() {
        let mut buffer: BytesMut = BytesMut::from(
            &b"5\r\nhello\r\n0\r\nTrailer: value\r\n\r\n"[..]
        );
        let mut decoder = ChunkedDecoder::new();
        
        let chunk = decoder.decode(&mut buffer).unwrap().unwrap();
        assert_eq!(chunk.as_bytes().unwrap(), &Bytes::copy_from_slice(b"hello"));
        
        let eof = decoder.decode(&mut buffer).unwrap().unwrap();
        assert!(eof.is_eof());
    }

    #[test]
    fn test_incomplete_chunk() {
        let mut buffer: BytesMut = BytesMut::from(&b"5\r\nhel"[..]);
        let mut decoder = ChunkedDecoder::new();
        
        // Should return Some when received partial chunk
        let chunk = decoder.decode(&mut buffer).unwrap();
        assert!(chunk.is_some());
        assert_eq!(chunk.unwrap().as_bytes().unwrap(), &Bytes::copy_from_slice(b"hel"));
        
        // Add the rest of the chunk
        buffer.extend_from_slice(b"lo\r\n0\r\n\r\n");
        
        let chunk = decoder.decode(&mut buffer).unwrap().unwrap();
        assert_eq!(chunk.as_bytes().unwrap(), &Bytes::copy_from_slice(b"lo"));
        
        let eof = decoder.decode(&mut buffer).unwrap().unwrap();
        assert!(eof.is_eof());
    }

    #[test]
    fn test_invalid_chunk_size() {
        let mut buffer: BytesMut = BytesMut::from(&b"xyz\r\n"[..]);
        let mut decoder = ChunkedDecoder::new();
        
        let result = decoder.decode(&mut buffer);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_crlf() {
        let mut buffer: BytesMut = BytesMut::from(&b"5\r\nhelloBad"[..]);
        let mut decoder = ChunkedDecoder::new();
        
        let chunk = decoder.decode(&mut buffer).unwrap().unwrap();
        assert_eq!(chunk.as_bytes().unwrap(), &Bytes::copy_from_slice(b"hello"));
        
        let result = decoder.decode(&mut buffer);
        assert!(result.is_err());
    }

    #[test]
    fn test_large_chunk() {
        // Create a large chunk (1MB)
        let size = 1024 * 1024;
        let mut data = Vec::with_capacity(size + 16);
        let headers = format!("{:x}\r\n", size).into_bytes();
        data.extend(headers);
        data.extend(vec![b'A'; size]);
        data.extend(b"\r\n0\r\n\r\n");
        
        let mut buffer = BytesMut::from(&data[..]);
        let mut decoder = ChunkedDecoder::new();
        
        let chunk = decoder.decode(&mut buffer).unwrap().unwrap();
        assert_eq!(chunk.as_bytes().unwrap().len(), size);
        assert!(chunk.as_bytes().unwrap().iter().all(|&b| b == b'A'));
        
        let eof = decoder.decode(&mut buffer).unwrap().unwrap();
        assert!(eof.is_eof());
    }

    #[test]
    fn test_zero_size_chunk() {
        let mut buffer: BytesMut = BytesMut::from(&b"0\r\n\r\n"[..]);
        let mut decoder = ChunkedDecoder::new();
        
        let eof = decoder.decode(&mut buffer).unwrap().unwrap();
        assert!(eof.is_eof());
    }
}
