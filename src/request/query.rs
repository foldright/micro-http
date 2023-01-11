use std::collections::HashMap;

pub struct Query<'a> {
    data: HashMap<&'a str, Value<'a>>,
}

#[derive(Debug, PartialEq)]
pub enum Value<'a> {
    Single(&'a str),
    Multi(Vec<&'a str>),
}

impl <'a> Query<'a> {
    pub fn get(&self, key: &str) -> Option<&'a Value> {
        self.data.get(key)
    }
}

impl<'a> From<&'a str> for Query<'a> {
    // a=1&c=2&b=3
    // ""
    // a=1&a=2
    // a&b&c=2
    fn from(str: &'a str) -> Self {
        let mut data: HashMap<&'a str, Value<'a>> = HashMap::new();

        if str.is_empty() {
            return Query { data };
        }

        for kv in str.split('&') {
            let (mut key, mut value) = (kv, "");
            if let Some((k, v)) = kv.split_once('=') {
                (key, value) = (k, v);
            }

            data.entry(key)
                .and_modify(|old_value| {
                    match old_value {
                        Value::Single(v) => {
                            *old_value = Value::Multi(vec![v, value]);
                        }
                        Value::Multi(ref mut vec) => {
                            vec.push(value);
                        }
                    }
                })
                .or_insert(Value::Single(value));
        }

        Query { data }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_from_empty_str() {
        let query = Query::from("");
        assert_eq!(query.data.len(), 0);
    }

    #[test]
    fn test_query_from_str() {
        {
            let query = Query::from("a=1&b=2");
            assert_eq!(query.data.len(), 2);
            assert_eq!(query.get("a"), Some(&Value::Single("1")));
            assert_eq!(query.get("b"), Some(&Value::Single("2")));
        }

        {
            let query = Query::from("a=&b=2&c&a=42");
            assert_eq!(query.data.len(), 3);
            assert_eq!(query.get("a"), Some(&Value::Multi(vec!["","42"])));
            assert_eq!(query.get("b"), Some(&Value::Single("2")));
            assert_eq!(query.get("c"), Some(&Value::Single("")));
        }
    }
}