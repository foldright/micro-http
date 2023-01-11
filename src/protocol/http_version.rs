use crate::protocol::error::ParseError;

#[derive(Debug, PartialEq)]
pub enum HttpVersion {
    Http1_1
}

impl TryFrom<&str> for HttpVersion {
    type Error = ParseError;

    fn try_from(str: &str) -> Result<Self, Self::Error> {
        match str {
            "HTTP/1.1" => Ok(Self::Http1_1),
            _ => Err(ParseError::InvalidRequest)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        let version = HttpVersion::try_from("HTTP/1.1");
        assert_eq!(version, Ok(HttpVersion::Http1_1));
    }

    #[test]
    fn test_from_invalid_str() {
        let version = HttpVersion::try_from("HTTP1.1");
        assert_eq!(version, Err(ParseError::InvalidVersion));
    }
}