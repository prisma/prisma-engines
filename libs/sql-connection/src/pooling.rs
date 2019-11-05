use quaint::{
    connector::{Queryable},
    error::Error as QueryError,
    pool::{CheckOut, Manage, Pool},
};

/// A handle to a database connection pool that works generically over a pool of a single
/// connection or a prisma query connection pool.
pub(crate) enum ConnectionPool<C, P>
where
    C: Queryable + Send + Sync,
    P: Manage<Resource = C>,
{
    Single(C),
    Pool(Pool<P>),
}

impl<C, P> ConnectionPool<C, P>
where
    C: Queryable + Send + 'static,
    P: Manage<Resource = C, Error = QueryError, CheckOut = CheckOut<P>> + Send + Sync,
{
    pub(crate) async fn get_connection<'a>(&'a self) -> Result<ConnectionHandle<'a, C, P>, QueryError> {
        match &self {
            ConnectionPool::Single(conn) => Ok(ConnectionHandle::Single(conn)),
            ConnectionPool::Pool(pool) => {
                let checkout: CheckOut<P> = pool.check_out().await?;
                Ok(ConnectionHandle::PoolCheckout(checkout))
            }
        }
    }
}

/// A handle to a single connection from a [`ConnectionPool`](/enum.ConnectionPool.html)).
pub(crate) enum ConnectionHandle<'a, C, P>
where
    C: Queryable + Send + Sync + 'static,
    P: Manage<Resource = C, Error = QueryError, CheckOut = CheckOut<P>> + Send + Sync,
{
    Single(&'a C),
    PoolCheckout(CheckOut<P>),
}

impl<'a, C, P> ConnectionHandle<'a, C, P>
where
    C: Queryable + Send,
    P: Manage<Resource = C, Error = QueryError, CheckOut = CheckOut<P>> + Send + Sync,
{
    pub(crate) fn as_queryable(&self) -> &dyn Queryable {
        match self {
            ConnectionHandle::Single(guard) => guard,
            ConnectionHandle::PoolCheckout(co) => co,
        }
    }
}
