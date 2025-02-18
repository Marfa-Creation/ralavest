use std::collections::HashMap;

use nom::{
    bytes::complete::{tag, take_till1},
    combinator::opt,
    multi::separated_list1,
    sequence::separated_pair,
    IResult, Parser,
};
use percent_encoding::percent_decode;
use serde_json::de::SliceRead;

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
pub struct CookieStore(pub HashMap<String, String>);

/// json extractor, return None if body of the request body not json
pub struct Json<T>(pub Option<T>);

impl<'a, T> FromRequest for Json<T>
where
    T: serde::de::DeserializeOwned,
{
    fn extract(req: Request, _: (Method, String)) -> Self {
        Json(
            T::deserialize(&mut serde_json::Deserializer::new(SliceRead::new(
                req.get_body(),
            )))
            .ok(),
        )
        // todo!()
    }
}

impl FromRequest for CookieStore {
    fn extract(req: Request, _: (Method, String)) -> Self {
        if let Some(cookie_value) = req.get_headers().get("Cookie") {
            // return CookieStore(cookie_value)
            if let Ok(cookies) = cookie_value_parser(cookie_value) {
                return CookieStore(cookies.1);
            }
        }
        return CookieStore(HashMap::new());
    }
}

fn cookie_value_parser(input: &str) -> IResult<&str, HashMap<String, String>> {
    return (
        separated_list1(
            tag("; "),
            separated_pair(
                take_till1(|i| i == '='),
                nom::character::char('='),
                take_till1(|i| i == ';'),
            ),
        ),
        opt(nom::character::char(';')),
    )
        .parse_complete(input)
        .map(
            |(remain, (cookies, _))| -> (&str, HashMap<String, String>) {
                (
                    remain,
                    std::collections::HashMap::from_iter(
                        cookies.iter().map(|(k, v)| (k.to_string(), v.to_string())),
                    ),
                )
            },
        );
}

impl FromRequest for Path {
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
    fn parse_cookie() {

        let cookies = CookieStore::extract(
            Request::try_from("GET / HTTP/1.1\r\nCookie: session=hello; msg=world".as_bytes())
                .unwrap(),
            (Method::get, String::from("/")),
        );

        assert_eq!(
            HashMap::from([("session", "hello"), ("msg", "world")])
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<HashMap<String, String>>(),
            cookies.0
        );
    }

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
