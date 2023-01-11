use std::collections::HashMap;
use crate::protocol::error::ParseError;

pub struct Headers<'a> {
    data: HashMap<&'a str, &'a str>,
}

impl<'a> Headers<'a> {
    pub fn get(&self, key: &str) -> Option<&'a str> {
        self.data.get(key).map(|value| *value)
    }
}

impl<'a> TryFrom<&'a str> for Headers<'a> {
    type Error = ParseError;

    fn try_from(str: &'a str) -> Result<Self, Self::Error> {
        let mut data: HashMap<&str, &str> = HashMap::new();

        if str.trim().is_empty() {
            return Ok(Headers { data });
        }

        for line in str.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let (key, value) = line.split_once(':').ok_or(ParseError::InvalidRequest)?;
            data.insert(key.trim(), value.trim());
        }

        Ok(Headers { data })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        let str = format!("{}\r\n{}\r\n{}",
                          "Host: 127.0.0.1:8080",
                          "User-Agent: curl/7.79.1",
                          "Accept: */*",
        );

        let headers = Headers::try_from(str.as_str()).unwrap();

        assert_eq!(headers.data.len(), 3);
        assert_eq!(headers.get("Host"), Some("127.0.0.1:8080"));
        assert_eq!(headers.get("User-Agent"), Some("curl/7.79.1"));
        assert_eq!(headers.get("Accept"), Some("*/*"));
        assert_eq!(headers.get("Encoding"), None);
    }
}

// Host: 127.0.0.1:8080
// User-Agent: curl/7.79.1
// Accept: */*