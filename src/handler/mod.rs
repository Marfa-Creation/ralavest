use std::marker::PhantomData;

use crate::{
    extractor::FromRequest,
    http::{IntoResponse, Method, Request, Response},
};

pub trait Handler {
    fn call(&self, req: Request, matched: (Method, String)) -> Response;
}

pub trait IntoHandler<Input> {
    type Handler: Handler;

    fn into_handler(self) -> Self::Handler;
}

// declare with same name cause i don't have idea
macro_rules! impl_all {
    ($($i:ident),*) => {
        impl<F, R, $($i),*> Handler for FunctionHandler<($($i,)*), F>
        where
            F: Fn($($i),*) -> R,
            R: IntoResponse,
            $($i: FromRequest),*
        {
            fn call(&self, _req: Request, _matched: (Method, String)) -> Response {
                return (self.f)($($i::extract(_req.clone(), _matched.clone())),*).into_response();
            }
        }
    };
}

//for now we only support max 5 extractor per handler
impl_all!();
impl_all!(P1);
impl_all!(P1, P2);
impl_all!(P1, P2, P3);
impl_all!(P1, P2, P3, P4);
impl_all!(P1, P2, P3, P4, P5);

macro_rules! impl_all {
    ($($i:ident),*) => {
        impl<F, R, $($i),*> IntoHandler<($($i,)*)> for F
        where
            F: Fn($($i),*) -> R,
            R: IntoResponse,
            $($i: FromRequest),*
        {
            type Handler = FunctionHandler<($($i,)*), Self>;

            fn into_handler(self) -> Self::Handler {
                FunctionHandler {
                    f: self,
                    marker: PhantomData::default(),
                }
            }
        }
    };
}

//for now we only support max 5 extractor per handler
impl_all!();
impl_all!(P1);
impl_all!(P1, P2);
impl_all!(P1, P2, P3);
impl_all!(P1, P2, P3, P4);
impl_all!(P1, P2, P3, P4, P5);

pub struct FunctionHandler<Input, F> {
    f: F,
    marker: PhantomData<fn() -> Input>,
}

impl Clone for Box<dyn Handler> {
    fn clone(&self) -> Self {
        return self.to_owned();
    }
}

