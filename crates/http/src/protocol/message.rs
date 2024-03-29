use bytes::{Buf, Bytes};

/// represent the request/response message
pub enum Message<T, Data: Buf = Bytes> {
    Header(T),
    Payload(PayloadItem<Data>),
}

/// payload item produced from payload decoder
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayloadItem<Data: Buf = Bytes> {
    Chunk(Data),
    Eof,
}

/// represent the payload size
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PayloadSize {
    Length(u64),
    Chunked,
    Empty,
}

impl PayloadSize {
    #[inline]
    pub fn is_chunked(&self) -> bool {
        matches!(self, PayloadSize::Chunked)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        matches!(self, PayloadSize::Empty)
    }
}

impl<T> Message<T> {
    pub fn is_payload(&self) -> bool {
        match self {
            Message::Header(_) => false,
            Message::Payload(_) => true,
        }
    }

    pub fn is_header(&self) -> bool {
        match self {
            Message::Header(_) => true,
            Message::Payload(_) => false,
        }
    }

    pub fn into_payload_item(self) -> Option<PayloadItem> {
        match self {
            Message::Header(_) => None,
            Message::Payload(payload_item) => Some(payload_item),
        }
    }
}

impl<T> From<Bytes> for Message<T> {
    fn from(bytes: Bytes) -> Self {
        Self::Payload(PayloadItem::Chunk(bytes))
    }
}

impl<D: Buf> PayloadItem<D> {
    pub fn is_eof(&self) -> bool {
        match self {
            PayloadItem::Chunk(_) => false,
            PayloadItem::Eof => true,
        }
    }

    pub fn is_chunk(&self) -> bool {
        match self {
            PayloadItem::Chunk(_) => true,
            PayloadItem::Eof => false,
        }
    }
}

impl PayloadItem {
    pub fn as_bytes(&self) -> Option<&Bytes> {
        match self {
            PayloadItem::Chunk(bytes) => Some(bytes),
            PayloadItem::Eof => None,
        }
    }

    pub fn as_mut_bytes(&mut self) -> Option<&mut Bytes> {
        match self {
            PayloadItem::Chunk(bytes) => Some(bytes),
            PayloadItem::Eof => None,
        }
    }

    pub fn into_bytes(self) -> Option<Bytes> {
        match self {
            PayloadItem::Chunk(bytes) => Some(bytes),
            PayloadItem::Eof => None,
        }
    }
}
