use bytes::Bytes;
use futures::future::Then;

/// represent the request/response message
pub enum Message<T> {
    Header(T),
    Payload(PayloadItem),
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

/// payload item produced from payload decoder
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayloadItem {
    Chunk(Bytes),
    Eof,
}

impl PayloadItem {
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
