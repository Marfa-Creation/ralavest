use std::marker::PhantomData;

use crate::{
    extractor::FromRequest,
    http::{IntoResponse, Request, Response},
};

pub trait Handler {
    fn call(&self, req: Request) -> Response;
}

pub trait IntoHandler<Input> {
    type Handler: Handler;

    fn into_handler(self) -> Self::Handler;
}

//TODO: make shorter code using macro
impl<F, R> IntoHandler<()> for F
where
    F: Fn() -> R,
    R: IntoResponse,
{
    type Handler = FunctionHandler<(), Self>;

    fn into_handler(self) -> Self::Handler {
        FunctionHandler {
            f: self,
            marker: PhantomData::default(),
        }
    }
}

impl<F, R, P1> IntoHandler<(P1,)> for F
where
    F: Fn(P1) -> R,
    R: IntoResponse,
    P1: FromRequest,
{
    type Handler = FunctionHandler<(P1,), Self>;

    fn into_handler(self) -> Self::Handler {
        FunctionHandler {
            f: self,
            marker: PhantomData::default(),
        }
    }
}

impl<F, R, P1, P2> IntoHandler<(P1, P2)> for F
where
    F: Fn(P1, P2) -> R,
    R: IntoResponse,
    P1: FromRequest,
    P2: FromRequest,
{
    type Handler = FunctionHandler<(P1, P2), Self>;

    fn into_handler(self) -> Self::Handler {
        FunctionHandler {
            f: self,
            marker: PhantomData::default(),
        }
    }
}
/////////////////////////////////////////////////////////////

pub struct FunctionHandler<Input, F> {
    f: F,
    marker: PhantomData<fn() -> Input>,
}

//TODO: make shorter code using macro
impl<F, R> Handler for FunctionHandler<(), F>
where
    F: Fn() -> R,
    R: IntoResponse,
{
    fn call(&self, _: Request) -> Response {
        (self.f)().into_response()
    }
}

impl<F, R, P1> Handler for FunctionHandler<(P1,), F>
where
    F: Fn(P1) -> R,
    R: IntoResponse,
    P1: FromRequest,
{
    fn call(&self, req: Request) -> Response {
        (self.f)(P1::extract(req)).into_response()
    }
}

impl<F, R, P1, P2> Handler for FunctionHandler<(P1, P2), F>
where
    F: Fn(P1, P2) -> R,
    R: IntoResponse,
    P1: FromRequest,
    P2: FromRequest,
{
    fn call(&self, req: Request) -> Response {
        (self.f)(P1::extract(req.clone()), P2::extract(req)).into_response()
    }
}
////////////////////////////////////////

impl Clone for Box<dyn Handler> {
    fn clone(&self) -> Self {
        return self.to_owned();
    }
}

impl<F, R> Handler for F
where
    R: IntoResponse,
    F: Fn() -> R + Clone,
{
    fn call(&self, _: Request) -> Response
    where
        Self: Sized,
    {
        return self().into_response();
    }
}
