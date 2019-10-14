use futures::future::{BoxFuture, FutureExt};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

pub struct DBIO<'a, T>(BoxFuture<'a, crate::Result<T>>);

impl<'a, T> DBIO<'a, T>
{
    pub fn new<F>(inner: F) -> Self
    where
        F: Future<Output = crate::Result<T>> + Send + 'a,
    {
        Self(inner.boxed())
    }
}

impl<'a, T> Future for DBIO<'a, T>
{
    type Output = crate::Result<T>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.as_mut().poll(ctx)
    }
}
