use crate::{BoxFuture, ConnectorResult};

/// An abstract host for a migration connector. It exposes IO that is not directly performed by the
/// connectors.
pub trait ConnectorHost: Sync + Send + 'static {
    /// Print to the console.
    fn print<'a>(&'a self, text: &'a str) -> BoxFuture<'a, ConnectorResult<()>>;
}

/// A no-op ConnectorHost.
#[derive(Debug, Clone)]
pub struct EmptyHost;

impl ConnectorHost for EmptyHost {
    fn print(&self, text: &str) -> BoxFuture<'_, ConnectorResult<()>> {
        // https://github.com/prisma/prisma/issues/11761
        assert!(text.ends_with('\n'));
        Box::pin(std::future::ready(Ok(())))
    }
}
