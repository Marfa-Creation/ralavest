use crate::http::{IntoResponse, Response};

pub trait Handler {
    fn call(&self) -> Response;
}

impl<F, R> Handler for F
where
    R: IntoResponse,
    F: Fn() -> R + Clone,
{
    fn call(&self) -> Response
    where
        Self: Sized,
    {
        return self().into_response();
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        return self;
    }
}

impl Clone for Box<dyn Handler> {
    fn clone(&self) -> Self {
        return self.to_owned();
    }
}
