use std::{
    collections::{BTreeMap, HashMap},
    fmt::Display,
};

#[derive(Debug, Clone)]
pub struct Request {
    method: Method,
    path: String,
    protocol: Protocol,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

//TODO: create constructor
impl Request {
    pub fn new(method: Method, path: String, protocol: Protocol) -> Self {
        return Request {
            method,
            path,
            protocol,
            headers: HashMap::new(),
            body: vec![],
        };
    }

    pub fn get_start_line<'s>(&'s self) -> String {
        return format!("{:?} {} {:?}", self.method, self.path, self.protocol);
    }

    pub fn get_method(&self) -> &Method {
        return &self.method;
    }

    pub fn get_path(&self) -> &str {
        return &self.path;
    }

    pub fn get_protocol(&self) -> &Protocol {
        return &self.protocol;
    }

    pub fn get_headers<'s>(&self) -> &HashMap<String, String> {
        return &self.headers;
    }

    pub fn get_body(&self) -> &Vec<u8> {
        return &self.body;
    }
}

impl TryFrom<&[u8]> for Request {
    type Error = ParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut value = lines(&trim(&trim(value, b' '), b'\0'));
        let start_line = String::from_utf8_lossy(value.get(0).ok_or(ParseError(
            "HTTP message cannot be built from empty string".to_string(),
        ))?);

        let method: Method = {
            let mut method = String::new();
            for i in start_line.as_bytes() {
                if i == &b' ' {
                    break;
                }

                method += String::from_utf8([*i].to_vec())
                    .expect("invalid converting u32 into char")
                    .as_str();
            }
            method.as_str().try_into().expect("unhandled error")
        };
        let path = {
            let mut path = String::new();
            for i in start_line
                .replace((method.to_string().clone() + " ").as_str(), "")
                .as_bytes()
            {
                if i == &b' ' {
                    break;
                }

                path += String::from_utf8([*i].to_vec())
                    .expect("invalid converting u32 into char")
                    .as_str();
            }
            path
        };

        let protocol = Protocol::try_from(
            start_line
                .replace(format!("{:?} {} ", method, path).as_str(), "")
                .as_str(),
        )?;

        // remove first line that has readed
        value.remove(0);
        let headers = {
            let mut headers = HashMap::<String, String>::new();
            for i in value.clone() {
                //only accept valid UTF8 header
                let i = String::from_utf8_lossy(&i);
                // based on MDN, empty line indicating the end of headers
                if i.is_empty() {
                    break;
                }

                // remove headers that has readed
                let (k, v) = i
                    .split_once(':')
                    .ok_or(ParseError("invalid header of HTTP message".to_string()))?;

                headers.insert(k.trim().to_string(), v.trim().to_string());

                value.retain(|e| String::from_utf8_lossy(e) != i);
            }

            headers
        };
        let body = {
            let mut body = vec![];
            for i in value {
                body.push(i);
            }
            body.concat()
        };

        Ok(Request {
            protocol,
            headers,
            body,
            path,
            method,
        })
    }
}

//TODO: make getter for field
#[derive(Debug)]
pub struct Response {
    protocol: Protocol,
    status_code: String,
    status_text: String,
    //NOTE: IDK is it slower than HashMap or not. for now i using this to make the unit test consistent
    headers: BTreeMap<String, String>,
    //TODO: maybe we'll use Option and make Body type
    body: Vec<u8>,
}

//TODO: make good constructor
impl Response {
    /// construct [Response] with minimal default value
    pub fn new() -> Self {
        let content = "<h1>Hello World</h1>";
        Self {
            protocol: Protocol::HTTP_1_1,
            status_code: 200.to_string(),
            status_text: "OK".to_string(),
            headers: BTreeMap::from([
                ("Content-Length".to_string(), content.len().to_string()),
                ("Content-Type".to_string(), "text/html".to_string()),
            ]),
            body: content.as_bytes().to_vec(),
        }
    }

    /// the difference from [new](Self::new) is [build](Self::build) has no default value
    pub fn build(protocol: Protocol, status_code: u32, status_text: impl ToString) -> Self {
        return Self {
            protocol,
            status_code: status_code.to_string(),
            status_text: status_text.to_string(),
            headers: BTreeMap::new(),
            body: vec![],
        };
    }

    //openregion: --> getter

    //endregion:  --> getter

    //openregion: --> setter

    pub fn set_protocol(mut self, protocol: Protocol) -> Self {
        self.protocol = protocol;
        return self;
    }

    pub fn set_status_code(mut self, status_code: u32) -> Self {
        self.status_code = status_code.to_string();
        return self;
    }

    pub fn set_status_text(mut self, status_text: impl ToString) -> Self {
        self.status_text = status_text.to_string();
        return self;
    }

    pub fn set_header(mut self, key: impl ToString, value: impl ToString) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        return self;
    }

    pub fn set_body(mut self, body: Vec<u8>) -> Self {
        self.body = body.clone();

        return self.set_header("Content-Length".to_string(), body.len().to_string());
    }

    //endregion:  --> setter

    pub fn raw(&self) -> Vec<u8> {
        let headers = {
            let mut headers = String::new();

            for (k, v) in self.headers.iter() {
                headers += format!("{}: {}\n", k, v).as_str();
            }
            headers
        };
        return [
            self.protocol.to_string().as_bytes(),
            b" ",
            &self.status_code.as_bytes(),
            b" ",
            &self.status_text.as_bytes(),
            b"\n",
            &headers.as_bytes(),
            b"\n",
            &self.body,
        ]
        .concat();
    }
}

impl TryFrom<&[u8]> for Response {
    type Error = ParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        // trim_matches for delete unused buffer
        let mut value = lines(&trim(&trim(value, b' '), b'\0'));
        // .trim()
        // .trim_matches('\0')
        // .lines()
        // .into_iter()
        // .collect::<Vec<&str>>();
        let start_line = String::from_utf8_lossy(value.get(0).ok_or(ParseError(
            "HTTP message cannot be built from empty string".to_string(),
        ))?);

        let protocol = {
            let mut protocol = String::new();
            for i in start_line.as_bytes() {
                if i == &b' ' {
                    break;
                }

                protocol += String::from_utf8([*i].to_vec())
                    .expect("invalid converting u32 into char")
                    .as_str();
            }
            Protocol::try_from(protocol.as_str())
        }?;

        let status_code = {
            let mut status_code = String::new();
            for i in start_line
                .replace((protocol.to_string().clone() + " ").as_str(), "")
                .as_bytes()
            {
                if i == &b' ' {
                    break;
                }

                status_code += String::from_utf8([*i].to_vec())
                    .expect("invalid converting u32 into char")
                    .as_str();
            }
            status_code
        };

        let status_text = start_line
            .clone()
            .replace(format!("{:?} {} ", protocol, status_code).as_str(), "");

        // remove first line that has readed
        value.remove(0);
        let headers = {
            let mut headers = BTreeMap::<String, String>::new();
            for i in value.clone() {
                let i = String::from_utf8_lossy(&i);
                // based on MDN, empty line indicating the end of headers
                if i.is_empty() {
                    break;
                }

                let (k, v) = i
                    .split_once(':')
                    .ok_or(ParseError("invalid header of HTTP message".to_string()))?;

                headers.insert(k.trim().to_string(), v.trim().to_string());
                // remove headers that has readed
                value.retain(|e| String::from_utf8_lossy(e) != i);
            }

            headers
        };
        let body = {
            let mut body = vec![];
            for i in value {
                body.push(i);
            }
            body.concat()
        };

        Ok(Response {
            headers,
            body,
            protocol,
            status_code,
            status_text,
        })
    }
}

pub trait IntoResponse {
    fn into_response(self) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        return self;
    }
}

impl<S> IntoResponse for S
where
    S: ToString,
{
    fn into_response(self) -> Response {
        return Response::new()
            .set_header("Content-Length", self.to_string().len())
            .set_header("Content-Type", "text/html")
            .set_body(self.to_string().as_bytes().to_vec());
    }
}

//TODO: make good ParseError to make error handling easier
#[derive(Debug)]
pub struct ParseError(pub String);

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl std::error::Error for ParseError {}

#[allow(non_camel_case_types)]
#[derive(PartialEq, Eq, Hash, Clone)]
pub enum Method {
    get,
    head,
    options,
    trace,
    put,
    delete,
    post,
    patch,
    connect,
}

impl std::fmt::Debug for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Method::get => "GET",
                Method::post => "POST",
                Method::put => "PUT",
                Method::delete => "DELETE",
                Method::patch => "PATCH",
                Method::head => "HEAD",
                Method::options => "OPTIONS",
                Method::trace => "TRACE",
                Method::connect => "CONNECT",
            }
        )
    }
}

impl ToString for Method {
    fn to_string(&self) -> String {
        match self {
            Method::get => "GET".to_string(),
            Method::post => "POST".to_string(),
            Method::put => "PUT".to_string(),
            Method::delete => "DELETE".to_string(),
            Method::patch => "PATCH".to_string(),
            Method::head => "HEAD".to_string(),
            Method::options => "OPTIONS".to_string(),
            Method::trace => "TRACE".to_string(),
            Method::connect => "CONNECT".to_string(),
        }
    }
}
impl TryFrom<&str> for Method {
    type Error = ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        use Method::*;
        Ok({
            match value.to_lowercase().as_str() {
                "get" => get,
                "head" => head,
                "options" => options,
                "trace" => trace,
                "put" => put,
                "delete" => delete,
                "post" => post,
                "patch" => patch,
                "connect" => connect,
                _ => {
                    return Err(ParseError(format!(
                        "failed to parse '{}' into http::Method",
                        value
                    )))
                }
            }
        })
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone)]
pub enum Protocol {
    HTTP_1_1,
    HTTP_2,
    HTTP_3,
}

impl TryFrom<&str> for Protocol {
    type Error = ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        use Protocol::*;
        return match value.to_lowercase().as_str() {
            "http/1.1" => Ok(HTTP_1_1),
            "http/2" => Ok(HTTP_2),
            "http/3" => Ok(HTTP_3),
            _ => Err(ParseError(
                "invalid parsing &str into HTTP protocol".to_string(),
            )),
        };
    }
}

impl ToString for Protocol {
    fn to_string(&self) -> String {
        return match self {
            Protocol::HTTP_1_1 => "HTTP/1.1".to_string(),
            Protocol::HTTP_2 => "HTTP/2".to_string(),
            Protocol::HTTP_3 => "HTTP/3".to_string(),
        };
    }
}

impl std::fmt::Debug for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(
            f,
            "{}",
            match self {
                Protocol::HTTP_1_1 => "HTTP/1.1",
                Protocol::HTTP_2 => "HTTP/2",
                Protocol::HTTP_3 => "HTTP/3",
            }
        );
    }
}
fn lines(value: &[u8]) -> Vec<Vec<u8>> {
    let mut result = vec![];
    let mut chunk = vec![];
    let mut iter = value.iter().peekable();
    loop {
        let now = iter.next();

        if now == None {
            break;
        }

        if (now == Some(&b'\r') && iter.next() == Some(&&b'\n')) || now == Some(&b'\n') {
            result.push(chunk.clone());
            chunk.clear();
        } else if false {
        } else {
            chunk.push(now.unwrap().clone());
        }
    }
    result.push(chunk);
    return result;
}
pub fn trim(value: &[u8], ch: u8) -> Vec<u8> {
    return {
        let mut read = false;
        let mut temp = vec![];

        for i in value {
            if *i != ch && read == false {
                read = true;
            }
            if read {
                temp.push(*i);
            }
        }

        read = false;
        let mut result = vec![];
        for i in temp.iter().rev() {
            if *i != ch && read == false {
                read = true;
            }
            if read {
                result.push(*i);
            }
        }
        result.reverse();
        temp.extend_from_slice(&temp.iter().map(|i| *i).rev().collect::<Vec<u8>>());
        result
    };
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn parse_string_to_request() {
        let msg = "POST /users HTTP/1.1
Host: example.com
Content-Type: application/json
Content-Length: 16

{\"status\": \"OK\"}
";

        let req = Request::try_from(msg.as_bytes()).unwrap();

        assert_eq!("POST /users HTTP/1.1".to_string(), req.get_start_line());

        assert_eq!(Method::post, req.method);

        assert_eq!("/users", req.path);

        assert_eq!("HTTP/1.1", req.protocol.to_string());

        let headers = HashMap::from([
            ("Host", "example.com"),
            ("Content-Type", "application/json"),
            ("Content-Length", "16"),
        ]);

        assert_eq!(
            headers,
            req.headers
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect::<HashMap<&str, &str>>()
        );

        assert_eq!("{\"status\": \"OK\"}".as_bytes(), req.body);
    }

    #[test]
    fn parse_string_to_response() {
        let msg = "HTTP/1.1 404 Not Found
Location: http://example.com/users/123
Content-Type: application/json

{
  \"message\": \"New user created\",
  \"user\": {
    \"id\": 123,
    \"firstName\": \"Example\",
    \"lastName\": \"Person\",
    \"email\": \"bsmth@example.com\"
  }
}";

        let res = Response::try_from(msg.as_bytes()).unwrap();

        assert_eq!("HTTP/1.1", res.protocol.to_string());

        assert_eq!("404", res.status_code);

        assert_eq!("Not Found", res.status_text);

        let headers = HashMap::from([
            ("Content-Type", "application/json"),
            ("Location", "http://example.com/users/123"),
        ]);

        assert_eq!(
            headers,
            res.headers
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect::<HashMap<&str, &str>>()
        );
    }

    //TODO: make good test for this, and ther's inconsistency in the test
    #[test]
    fn parse_response_to_string() {
        let msg = "HTTP/1.1 201 Created
Content-Type: application/json
Location: http://example.com/users/123

{
  \"message\": \"New user created\",
  \"user\": {
    \"id\": 123,
    \"firstName\": \"Example\",
    \"lastName\": \"Person\",
    \"email\": \"bsmth@example.com\"
  }
}
";

        let res = Response {
            protocol: Protocol::HTTP_1_1,
            status_code: "201".to_string(),
            status_text: "Created".to_string(),
            headers: BTreeMap::from([
                ("Content-Type", "application/json"),
                ("Location", "http://example.com/users/123"),
            ])
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
            body: "{
  \"message\": \"New user created\",
  \"user\": {
    \"id\": 123,
    \"firstName\": \"Example\",
    \"lastName\": \"Person\",
    \"email\": \"bsmth@example.com\"
  }
}"
            .to_string()
            .as_bytes()
            .to_vec(),
        };

        assert_eq!(msg.trim().as_bytes(), trim(&res.raw(), b' '));
    }
}
