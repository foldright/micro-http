use bytes::{Buf, Bytes};

/// Represents a HTTP message that can either be a header or payload.
///
/// This enum is used to handle both request and response messages in the HTTP protocol.
/// The generic parameter `T` typically represents the header type (request or response header),
/// while `Data` represents the type of the payload data (defaults to `Bytes`).
pub enum Message<T, Data: Buf = Bytes> {
    /// Contains the header information of type `T`
    Header(T),
    /// Contains a chunk of payload data or EOF marker
    Payload(PayloadItem<Data>),
}

/// Represents an item in the HTTP message payload stream.
///
/// This enum is used by the payload decoder to produce either data chunks
/// or signal the end of the payload stream (EOF).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayloadItem<Data: Buf = Bytes> {
    /// A chunk of payload data
    Chunk(Data),
    /// Marks the end of the payload stream
    Eof,
}

/// Represents the size information of an HTTP payload.
///
/// This enum is used to determine how the payload should be processed:
/// - Known length: Process exact number of bytes
/// - Chunked: Process using chunked transfer encoding
/// - Empty: No payload to process
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PayloadSize {
    /// Payload with known length in bytes
    Length(u64),
    /// Payload using chunked transfer encoding
    Chunked,
    /// Empty payload (no body)
    Empty,
}

impl PayloadSize {
    /// Returns true if the payload uses chunked transfer encoding
    #[inline]
    pub fn is_chunked(&self) -> bool {
        matches!(self, PayloadSize::Chunked)
    }

    /// Returns true if the payload is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        matches!(self, PayloadSize::Empty)
    }
}

impl<T> Message<T> {
    /// Returns true if this message contains payload data
    #[inline]
    pub fn is_payload(&self) -> bool {
        matches!(self, Message::Payload(_))
    }

    /// Returns true if this message contains header information
    #[inline]
    pub fn is_header(&self) -> bool {
        matches!(self, Message::Header(_))
    }

    /// Converts the message into a PayloadItem if it contains payload data
    ///
    /// Returns None if the message contains header information
    pub fn into_payload_item(self) -> Option<PayloadItem> {
        match self {
            Message::Header(_) => None,
            Message::Payload(payload_item) => Some(payload_item),
        }
    }
}

/// Converts bytes into a Message
///
/// This allows bytes to be directly converted into a Message for sending payload data.
/// The generic type T is unused since this only creates payload messages.
impl<T> From<Bytes> for Message<T> {
    fn from(bytes: Bytes) -> Self {
        Self::Payload(PayloadItem::Chunk(bytes))
    }
}

impl<D: Buf> PayloadItem<D> {
    /// Returns true if this item represents the end of the payload stream
    #[inline]
    pub fn is_eof(&self) -> bool {
        matches!(self, PayloadItem::Eof)
    }

    /// Returns true if this item contains chunk data
    #[inline]
    pub fn is_chunk(&self) -> bool {
        matches!(self, PayloadItem::Chunk(_))
    }
}

impl PayloadItem {
    /// Returns a reference to the contained bytes if this is a Chunk
    ///
    /// Returns None if this is an EOF marker
    pub fn as_bytes(&self) -> Option<&Bytes> {
        match self {
            PayloadItem::Chunk(bytes) => Some(bytes),
            PayloadItem::Eof => None,
        }
    }

    /// Returns a mutable reference to the contained bytes if this is a Chunk
    ///
    /// Returns None if this is an EOF marker
    pub fn as_mut_bytes(&mut self) -> Option<&mut Bytes> {
        match self {
            PayloadItem::Chunk(bytes) => Some(bytes),
            PayloadItem::Eof => None,
        }
    }

    /// Consumes the PayloadItem and returns the contained bytes if this is a Chunk
    ///
    /// Returns None if this is an EOF marker
    pub fn into_bytes(self) -> Option<Bytes> {
        match self {
            PayloadItem::Chunk(bytes) => Some(bytes),
            PayloadItem::Eof => None,
        }
    }
}
