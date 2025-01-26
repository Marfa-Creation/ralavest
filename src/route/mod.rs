use core::str;
use std::{collections::HashMap, io, sync::Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{
    handler::{Handler, IntoHandler},
    http::{self, Method, Request, Response},
};

pub async fn serve<L>(listener: L, router: Router) -> io::Result<()>
where
    L: Into<tokio::net::TcpListener>,
{
    let router = Arc::new(router);

    let listener: tokio::net::TcpListener = listener.into();

    let rt = tokio::runtime::Builder::new_multi_thread().build().unwrap();

    loop {
        let router = router.clone();

        // TODO: question mark operator may cause panic(in spawned async task)
        // because question mark operator return into the `spawn` callback
        // instead of returning the error to `serve` function
        let (mut stream, _) = listener.accept().await?;
        rt.spawn(async move {
            let mut buff = [0; 1024];
            let mut vec_buff = vec![];

            if let Ok(_) = stream.read(&mut buff).await {
                if let Ok(_) = String::from_utf8(buff.into()) {
                    loop {
                        vec_buff.push(buff);

                        // if the last element of buff is not '\0'(null)
                        // it's mean the buffer already fill out
                        if buff.last().unwrap() != &b'\0' {
                            buff = [0; 1024];
                            stream.read(&mut buff).await?;
                        } else {
                            break;
                        }
                    }

                    if let Ok(req) = Request::try_from(vec_buff.concat().as_slice()) {
                        // single route can have multiple method to handled(which mean has multiple handler too)
                        'outer: for ((router_route, router_methods), router_handlers) in router
                            .routes
                            .iter()
                            .zip(router.method_routers.iter().map(|i| &i.methods).clone())
                            .zip(router.method_routers.iter().map(|i| &i.handlers).map(|h| h))
                        {
                            // match request with route,
                            // only taking path from user without taking the query param
                            // example: /login?usr=admin&pw=admin1234 -> /login
                            if req
                                .get_path()
                                .split_once("?")
                                .unwrap_or((req.get_path(), ""))
                                .0
                                == router_route
                            {
                                //match request with method
                                for (router_method, router_handler) in
                                    router_methods.into_iter().zip(router_handlers)
                                {
                                    if req.get_method() == router_method {
                                        stream
                                            .write_all(
                                                router_handler.call(req.clone()).raw().as_slice(),
                                            )
                                            .await?;

                                        // break to avoid triggering "Method Not Allowed"
                                        break 'outer;
                                    }
                                }

                                stream
                                    .write_all(
                                        Response::new()
                                            .set_protocol(http::Protocol::HTTP_1_1)
                                            .set_status_code(405)
                                            .set_status_text("Method Not Allowed")
                                            .set_header(
                                                "Allowed",
                                                router_methods
                                                    .iter()
                                                    .map(|method| format!("{:?}", method))
                                                    .collect::<Vec<String>>()
                                                    .join(", "),
                                            )
                                            .set_body(vec![])
                                            .raw()
                                            .as_slice(),
                                    )
                                    .await?;
                            }
                        }

                        stream
                            .write_all(router.fallback.call(req).raw().as_slice())
                            .await
                            .unwrap_or(());
                        // response "Bad Request" if the parser(`try_from` function) return Err
                    } else {
                        stream
                            .write_all(
                                Response::new()
                                    .set_protocol(http::Protocol::HTTP_1_1)
                                    .set_status_code(400)
                                    .set_status_text("Bad Request")
                                    .raw()
                                    .as_slice(),
                            )
                            .await?;
                    }
                }
            } else {
                stream
                    .write_all(
                        Response::new()
                            .set_protocol(http::Protocol::HTTP_1_1)
                            .set_status_code(400)
                            .set_status_text("Bad Request")
                            .set_header("Content-Type", "application/json")
                            .set_body(
                                "\"message\": \"invalid encoding request into UTF8\""
                                    .as_bytes()
                                    .to_vec(),
                            )
                            .raw()
                            .as_slice(),
                    )
                    .await?;
            }

            return Ok::<(), std::io::Error>(());
        });
    }
}

pub struct Router {
    routes: Vec<String>,
    method_routers: Vec<MethodRouter>,
    fallback: Box<dyn Handler + Send + Sync>,
}

impl Router {
    //TODO: encode non UTF8 route into UTF8
    //TODO: percent encoding
    pub fn route(mut self, route: &str, method_router: MethodRouter) -> Self {
        if route.as_bytes().get(0).unwrap_or(&b'/') != &b'/' {
            panic!("route must start with '/' character");
        }
        if self.routes.contains(&route.to_string()) {
            panic!("route '{}' is already exist", route);
        }

        self.routes.push(route.to_string());

        self.method_routers.push(method_router);

        return self;
    }

    pub fn new() -> Self {
        return Router {
            routes: vec![],
            method_routers: vec![],
            fallback: Box::new(|| {
                Response::new()
                                    .set_protocol(http::Protocol::HTTP_1_1)
                                    .set_status_code(404)
                                    .set_status_text("Not Found".to_string())
                                    .set_body((|| {
                                        if cfg!(debug_assertions){
                                            return "<h1>fallback</h1><p>this handler is called when no route is match, change this handler using Router::fallback</p>".as_bytes().to_vec();
                                        }
                                        "<h1>Not Found</1>".as_bytes().to_vec()})())
            }),
        };
    }

    pub fn fallback<H>(mut self, handler: H) -> Self
    where
        H: Handler + Send + Sync + 'static,
    {
        self.fallback = Box::new(handler);

        return self;
    }
}

#[allow(dead_code)]
//TODO: need to reviewed because lack of my knowledge about percent encoding
fn percent_encoding(mut string: String) -> String {
    let list = HashMap::from([
        (":", "%3A"),
        ("/", "%2F"),
        ("?", "%3F"),
        ("#", "%23"),
        ("[", "%5B"),
        ("]", "%5D"),
        ("@", "%40"),
        ("!", "%21"),
        ("$", "%24"),
        ("&", "%26"),
        ("'", "%27"),
        ("(", "%28"),
        (")", "%29"),
        ("*", "%2A"),
        ("+", "%2B"),
        (",", "%2C"),
        (";", "%3B"),
        ("=", "%3D"),
        ("%", "%25"),
        (" ", "%20"),
    ]);

    for (unencoded, encoded) in list.iter() {
        string = string.replace(unencoded, encoded);
    }

    return string;
}

pub struct MethodRouter {
    methods: Vec<http::Method>,
    handlers: Vec<Box<dyn Handler + Send + Sync>>,
}

macro_rules! method_for_all_http_method {
    ($($i:ident),*) => {
        impl MethodRouter {
            $(
                pub fn $i<I, H>(mut self, handler: impl IntoHandler<I, Handler = H>) -> Self
                where
                    H: Handler + Send + Sync + 'static
                {
                    if self.methods.contains(&Method::$i) {
                        panic!("route cannot have multiple handler for single method ");
                    }

                    self.methods.push(Method::$i);

                    self.handlers.push(Box::new(handler.into_handler()));

                    return self;
                }
            )*
        }
    };
}

method_for_all_http_method!(get, head, options, trace, put, delete, post, patch, connect);

macro_rules! func_for_all_http_method {
    ($($i:ident),*) => {
        $(
            pub fn $i<I, H>(handler: impl IntoHandler<I, Handler = H>) -> MethodRouter
            where
                H: Handler + Send + Sync + 'static
            {
                return MethodRouter {
                    methods: vec![Method::$i],
                    handlers: Vec::from(
                        [Box::new(handler.into_handler()) as Box<dyn Handler + Send + Sync>; 1],
                    ),
                }
            }
        )*
    };
}

func_for_all_http_method!(get, head, options, trace, put, delete, post, patch, connect);
