#![deny(warnings)]
#![macro_use]
extern crate failure_derive;

pub mod error;
pub mod filter;

mod compare;
mod interface;
mod query_arguments;
mod write_args;

pub use compare::*;
pub use filter::*;
pub use interface::*;
pub use query_arguments::*;
pub use write_args::*;

use futures::future::{BoxFuture, FutureExt};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};
use metrics_wrappers::timing;

pub type Result<T> = std::result::Result<T, error::ConnectorError>;

pub struct IO<'a, T> {
    inner: BoxFuture<'a, crate::Result<T>>,
    start_time: Instant,
}

impl<'a, T> IO<'a, T> {
    pub fn new<F>(inner: F) -> Self
    where
        F: Future<Output = crate::Result<T>> + Send + 'a,
    {
        Self {
            inner: inner.boxed(),
            start_time: Instant::now(),
        }
    }
}

impl<'a, T> Future for IO<'a, T> {
    type Output = crate::Result<T>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.inner.as_mut().poll(ctx) {
            Poll::Ready(t) => {
                timing!("query-engine.sql.query_time", self.start_time, Instant::now());
                Poll::Ready(t)
            }
            not_ready => not_ready,
        }
    }
}
