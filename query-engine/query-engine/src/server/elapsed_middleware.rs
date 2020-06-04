use futures::future::BoxFuture;
use tide::{Middleware, Next, Request};

use std::time::Instant;

/// Middleware to set the `X-Elapsed` header.
#[derive(Debug, Clone)]
pub(crate) struct ElapsedMiddleware {
    _priv: (),
}

impl ElapsedMiddleware {
    /// Creates a new `ElapsedMiddleware`.
    pub fn new() -> Self {
        Self { _priv: () }
    }
}

impl<State: Send + Sync + 'static> Middleware<State> for ElapsedMiddleware {
    fn handle<'a>(&'a self, cx: Request<State>, next: Next<'a, State>) -> BoxFuture<'a, tide::Result> {
        Box::pin(async move {
            let start = Instant::now();
            let mut res = next.run(cx).await?;
            let elapsed = Instant::now().duration_since(start).as_micros() as u64;
            res.insert_header("x-elapsed", format!("{}", elapsed));
            Ok(res)
        })
    }
}
