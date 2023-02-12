mod body;

use bytes::Bytes;
pub use body::ReqBody;


/// payload item produced from payload decoder
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayloadItem {
    Chunk(Bytes),
    Eof,
}