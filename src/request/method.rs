use crate::request::error::ParseError;

#[derive(Debug, PartialEq)]
pub enum Method {
    GET,
}

impl TryFrom<&str> for Method {
    type Error = ParseError;

    fn try_from(str: &str) -> Result<Self, Self::Error> {
        match str {
            "GET" => Ok(Self::GET),
            _ => Err(ParseError::InvalidRequest),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_from() {
        let result = Method::try_from("GET");
        assert_eq!(result, Ok(Method::GET));
    }

    #[test]
    fn test_method_from_error() {
        {
            let result = Method::try_from("get");
            assert_eq!(result, Err(ParseError::InvalidRequest));
        }

        {
            let result = Method::try_from("");
            assert_eq!(result, Err(ParseError::InvalidRequest));
        }
    }
}