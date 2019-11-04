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
};

pub type Result<T> = std::result::Result<T, error::ConnectorError>;

pub struct IO<'a, T>(BoxFuture<'a, crate::Result<T>>);

impl<'a, T> IO<'a, T> {
    pub fn new<F>(inner: F) -> Self
    where
        F: Future<Output = crate::Result<T>> + Send + 'a,
    {
        Self(inner.boxed())
    }
}

impl<'a, T> Future for IO<'a, T> {
    type Output = crate::Result<T>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.as_mut().poll(ctx)
    }
}
