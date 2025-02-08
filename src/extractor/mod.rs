use std::collections::HashMap;

use percent_encoding::percent_decode;

use crate::http::{Method, Request};

/// representing type that can be constructed using Request.
/// `req` is representing Request from the client.
/// `matched` is representing the matched route
pub trait FromRequest {
    fn extract(req: Request, matched: (Method, String)) -> Self
    where
        Self: Sized;
}

pub struct Query(pub HashMap<String, String>);
pub struct Path(pub HashMap<String, String>);

impl FromRequest for Path {
    //TODO: handle error correctly
    fn extract(req: Request, matched: (Method, String)) -> Self {
        let Ok(client_url) = url::Url::from_file_path(
            percent_decode(req.get_path().as_bytes())
                .decode_utf8_lossy()
                .to_string(),
        ) else {
            return Path(HashMap::new());
        };
        let Ok(matched_url) = url::Url::from_file_path(matched.1.as_str()) else {
            return Path(HashMap::new());
        };

        let client_seg = client_url.path_segments().unwrap();
        let matched_seg = matched_url.path_segments().unwrap();

        if client_seg.clone().count() != matched_seg.clone().count() {
            return Path(HashMap::new());
        }

        let mut result = HashMap::new();

        for (client_seg, matched_seg) in client_seg.into_iter().zip(matched_seg) {
            if let Some(name) = regex::Regex::new(r"(%7B)(.*+)(%7D)")
                .unwrap()
                .captures_iter(matched_seg)
                .next()
            {
                result.insert(
                    name[2].trim().to_string(),
                    percent_decode(client_seg.as_bytes())
                        .decode_utf8_lossy()
                        .to_string(),
                );
            }
        }

        return Path(result);
    }
}

impl FromRequest for Query {
    fn extract(req: Request, _: (Method, String)) -> Self {
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
    fn extract(req: Request, _: (Method, String)) -> Self {
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

        let query = <Query as FromRequest>::extract(req, (Method::get, String::from("/")));

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
