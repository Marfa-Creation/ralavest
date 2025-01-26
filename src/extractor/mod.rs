use std::collections::HashMap;

use crate::http::Request;

/// representing type that can be constructed using Request
pub trait FromRequest {
    fn extract(req: Request) -> Self
    where
        Self: Sized;
}

pub struct Query(pub HashMap<String, String>);

impl FromRequest for Query {
    fn extract(req: Request) -> Self {
        match req.get_path().split_once("?") {
            Some(str) => {
                let mut v_area = false;
                let mut k = String::new();
                let mut v = String::new();
                let mut hashmap = HashMap::new();

                let mut str = str
                    .1
                    .split_once("#")
                    .unwrap_or_else(|| (str.1, ""))
                    .0
                    .chars()
                    .peekable();

                while let Some(i) = str.next() {
                    if i == '=' {
                        v_area = true;
                        continue;
                    }

                    if i == '&' {
                        v_area = false;
                        hashmap.insert(k.clone(), v.clone());
                        k.clear();
                        v.clear();
                        continue;
                    }

                    if v_area {
                        v += i.to_string().as_str();
                    } else {
                        k += i.to_string().as_str();
                    }

                    if str.peek() == None {
                        hashmap.insert(k.clone(), v.clone());
                        k.clear();
                        v.clear();
                    }
                }
                return Query(hashmap);
            }
            None => {
                return Query(HashMap::new());
            }
        }
    }
}

impl FromRequest for Request {
    fn extract(req: Request) -> Self {
        return req;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_query() {
        let req =
            Request::try_from("GET /login?usr=admin&pw=admin1234#end-page HTTP/1.1".as_bytes())
                .unwrap();

        let query = <Query as FromRequest>::extract(req);

        assert_eq!(
            HashMap::from([("usr", "admin"), ("pw", "admin1234")]),
            query
                .0
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect()
        )
    }
}
