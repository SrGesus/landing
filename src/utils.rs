use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use axum::response::{IntoResponse, Response};
use pin_project_lite::pin_project;
use tower::Service;

// pub trait AxumService: Sized {

//     fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
//         todo!()
//     }

//     fn as_service(self) -> IntoAxumService<Self> {
//         IntoAxumService(self)
//     }
// }

// pub struct IntoAxumService<S: AxumService>(S);

// impl<S: AxumService> Service<Request> for IntoAxumService<S> {
//     type Response = Response;
//     type Error = Infallible;
//     type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

//     fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         todo!()
//     }

//     fn call(&mut self, req: Request) -> Self::Future {
//         todo!()
//     }

// }

#[derive(Clone)]
pub(crate) struct MapIntoResponse<S> {
    inner: S,
}

impl<S> MapIntoResponse<S> {
    pub(crate) fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<B, S> Service<http::Request<B>> for MapIntoResponse<S>
where
    S: Service<http::Request<B>>,
    S::Response: IntoResponse,
{
    type Response = Response;
    type Error = S::Error;
    type Future = MapIntoResponseFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: http::Request<B>) -> Self::Future {
        MapIntoResponseFuture {
            inner: self.inner.call(req),
        }
    }
}

pin_project! {
    pub(crate) struct MapIntoResponseFuture<F> {
        #[pin]
        inner: F,
    }
}

impl<F, T, E> Future for MapIntoResponseFuture<F>
where
    F: Future<Output = Result<T, E>>,
    T: IntoResponse,
{
    type Output = Result<Response, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let res = ready!(self.project().inner.poll(cx)?);
        Poll::Ready(Ok(res.into_response()))
    }
}
