use std::{
    collections::{HashMap, HashSet},
    io,
    sync::Arc,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{
    handler::Handler,
    http::{self, Method, Request, Response},
};

pub async fn serve<L>(listener: L, router: Router) -> io::Result<()>
where
    L: Into<tokio::net::TcpListener>
{
    let router = Arc::new(router);

    let listener: tokio::net::TcpListener = listener.into();

    let rt = tokio::runtime::Builder::new_multi_thread().build().unwrap();

    loop {
        let router = router.clone();

        let (mut stream, _) = listener.accept().await?;
        rt.spawn(async move {
            println!("outer task spawn");
            let mut buff = [0; 1024];

            if let Ok(_) = stream.read(&mut buff).await {
                if let Ok(v) = String::from_utf8(buff.into()) {
                    if let Ok(req) = Request::try_from(v.as_str()) {

                        // single route can have multiple method to handled(which mean has multiple handler too)
                        for ((router_route, router_methods), router_handlers) in router
                            .routes
                            .iter()
                            .zip(router.method_routers.iter().map(|i| &i.methods).clone())
                            .zip(router.method_routers.iter().map(|i| &i.handlers).map(|h| h))
                        {

                            // match request with route
                            if req.get_path() == router_route {

                                //match request with method
                                for (router_method, router_handler) in
                                    router_methods.into_iter().zip(router_handlers)
                                {
                                    if req.get_method() == router_method {
                                        println!(
                                            "method match {:?} {}",
                                            router_method, router_route
                                        );
                                        stream
                                            .write_all(router_handler.call().raw().as_bytes())
                                            .await?;

                                        // break to avoid triggering "Method Not Allowed"
                                        break;
                                    }

                                }

                                stream.write_all(Response::new()
                                    .set_protocol("HTTP/1.1").
                                    set_status_code(405)
                                    .set_status_text("Method Not Allowed")
                                    .set_header(
                                        "Allowed", 
                                       router_methods
                                                .iter()
                                                .map(|method| format!("{:?}", method))
                                                .collect::<Vec<String>>()
                                                .join(", "))
                                    .set_body("")
                                    .raw()
                                    .as_bytes()).await?;
                            }
                        }

                        //TODO: implement fallback(not found)
                        stream
                            .write_all(
                                Response::new()
                                    .set_protocol("HTTP/1.1".to_string())
                                    .set_status_code(404)
                                    .set_status_text("Not Found".to_string())
                                    .set_body((|| {
                                        if cfg!(debug_assertions){
                                            return "<h1>fallback</h1><p>this handler is called when no route is match, change this handler using Router::fallback</p>".to_string();
                                        }
                                        "<h1>Not Found</1>".to_string()})())
                                    .raw()
                                    .as_bytes(),
                            )
                            .await
                            .unwrap();
                    }
                }
            }

            // stream.write_all(html.as_bytes()).await.unwrap();
            return Ok::<(), std::io::Error>(());
        });
    }
}

pub struct Router {
    routes: HashSet<String>,
    // methods: Vec<Vec<Method>>,
    // handlers: Vec<Vec<Box<dyn Handler + Send + Sync>>>,
    method_routers: Vec<MethodRouter>,
}

impl Router {
    //TODO: encode non UTF8 route into UTF8
    //TODO: percent encoding
    pub fn route(mut self, route: &str, method_router: MethodRouter) -> Self {
        if route.as_bytes().get(0).unwrap_or(&b'/') != &b'/' {
            panic!("route must start with '/' character");
        }

        if self.routes.insert(route.to_string()) == false {
            panic!("route '{}' is already exist", route);
        }

        self.method_routers.push(method_router);

        // self.methods.push(Vec::from(
        //     method_router.methods.into_iter().collect::<Vec<Method>>(),
        // ));

        // self.handlers.push(method_router.handlers);

        return self;
    }

    pub fn new() -> Self {
        return Router {
            routes: HashSet::new(),
            method_routers: vec![], // methods: vec![],
                                    // handlers: vec![],
        };
    }
}

#[allow(dead_code)]
//TODO: need to revied because lack of my knowledge about percent encoding
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

// this struct must not exposed to public
pub struct MethodRouter {
    methods: Vec<http::Method>,
    handlers: Vec<Box<dyn Handler + Send + Sync>>,
}

//TODO: i think repeating method can be cut into macro
impl MethodRouter {
    pub fn get<H: Handler + Send + Sync + 'static>(mut self, handler: H) -> Self {
        if self.methods.contains(&Method::Get) {
            panic!("route cannot have multiple handler for single method ");
        }

        self.methods.push(Method::Get);

        self.handlers.push(Box::new(handler));

        return self;
    }

    
    pub fn head<H: Handler + Send + Sync + 'static>(mut self, handler: H) -> Self {
        if self.methods.contains(&Method::Head) {
            panic!("route cannot have multiple handler for single method ");
        }

        self.methods.push(Method::Head);

        self.handlers.push(Box::new(handler));

        return self;
    }


    pub fn options<H: Handler + Send + Sync + 'static>(mut self, handler: H) -> Self {
        if self.methods.contains(&Method::Options) {
            panic!("route cannot have multiple handler for single method ");
        }

        self.methods.push(Method::Options);

        self.handlers.push(Box::new(handler));

        return self;
    }

    
    pub fn trace<H: Handler + Send + Sync + 'static>(mut self, handler: H) -> Self {
        if self.methods.contains(&Method::Trace) {
            panic!("route cannot have multiple handler for single method ");
        }

        self.methods.push(Method::Trace);

        self.handlers.push(Box::new(handler));

        return self;
    }
    
    pub fn put<H: Handler + Send + Sync + 'static>(mut self, handler: H) -> Self {
        if self.methods.contains(&Method::Put) {
            panic!("route cannot have multiple handler for single method ");
        }

        self.methods.push(Method::Put);

        self.handlers.push(Box::new(handler));

        return self;
    }
    
    pub fn delete<H: Handler + Send + Sync + 'static>(mut self, handler: H) -> Self {
        if self.methods.contains(&Method::Delete) {
            panic!("route cannot have multiple handler for single method ");
        }

        self.methods.push(Method::Delete);

        self.handlers.push(Box::new(handler));

        return self;
    }
    
    pub fn post<H: Handler + Send + Sync + 'static>(mut self, handler: H) -> Self {
        if self.methods.contains(&Method::Post) {
            panic!("route cannot have multiple handler for single method ");
        }

        self.methods.push(Method::Post);

        self.handlers.push(Box::new(handler));

        return self;
    }

    pub fn patch<H: Handler + Send + Sync + 'static>(mut self, handler: H) -> Self {
        if self.methods.contains(&Method::Patch) {
            panic!("route cannot have multiple handler for single method ");
        }

        self.methods.push(Method::Patch);

        self.handlers.push(Box::new(handler));

        return self;
    }

    pub fn connect<H: Handler + Send + Sync + 'static>(mut self, handler: H) -> Self {
        if self.methods.contains(&Method::Connect) {
            panic!("route cannot have multiple handler for single method ");
        }

        self.methods.push(Method::Connect);

        self.handlers.push(Box::new(handler));

        return self;
    }
}

pub fn get<H: Handler + Send + Sync + 'static>(handler: H) -> MethodRouter {
    return MethodRouter {
        methods: vec![Method::Get],
        handlers: Vec::from([Box::new(handler) as Box<dyn Handler + Send + Sync>; 1]),
    };
}

pub fn post<H: Handler + Send + Sync + 'static>(handler: H) -> MethodRouter {
    return MethodRouter {
        methods: vec![Method::Post],
        handlers: Vec::from([Box::new(handler) as Box<dyn Handler + Send + Sync>; 1]),
    };
}

pub fn update<H: Handler + Send + Sync + 'static>(handler: H) -> MethodRouter {
    return MethodRouter {
        methods: vec![Method::Put],
        handlers: Vec::from([Box::new(handler) as Box<dyn Handler + Send + Sync>; 1]),
    };
}

pub fn delete<H: Handler + Send + Sync + 'static>(handler: H) -> MethodRouter {
    return MethodRouter {
        methods: vec![Method::Delete],
        handlers: Vec::from([Box::new(handler) as Box<dyn Handler + Send + Sync>; 1]),
    };
}

pub fn patch<H: Handler + Send + Sync + 'static>(handler: H) -> MethodRouter {
    return MethodRouter {
        methods: vec![Method::Patch],
        handlers: Vec::from([Box::new(handler) as Box<dyn Handler + Send + Sync>; 1]),
    };
}
