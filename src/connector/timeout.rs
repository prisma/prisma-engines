use crate::error::{Error, ErrorKind};
use futures::Future;
use std::time::Duration;

pub async fn connect<T, F, E>(duration: Option<Duration>, f: F) -> crate::Result<T>
where
    F: Future<Output = std::result::Result<T, E>>,
    E: Into<Error>,
{
    timeout(duration, f, || Error::builder(ErrorKind::ConnectTimeout).build()).await
}

pub async fn socket<T, F, E>(duration: Option<Duration>, f: F) -> crate::Result<T>
where
    F: Future<Output = std::result::Result<T, E>>,
    E: Into<Error>,
{
    timeout(duration, f, || Error::builder(ErrorKind::SocketTimeout).build()).await
}

#[cfg(any(feature = "mssql", feature = "postgresql", feature = "mysql"))]
async fn timeout<T, F, E, EF>(duration: Option<Duration>, f: F, e_f: EF) -> crate::Result<T>
where
    F: Future<Output = std::result::Result<T, E>>,
    EF: FnOnce() -> Error,
    E: Into<Error>,
{
    match duration {
        Some(duration) => match tokio::time::timeout(duration, f).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(err)) => Err(err.into()),
            Err(_) => Err(e_f()),
        },
        None => match f.await {
            Ok(result) => Ok(result),
            Err(err) => Err(err.into()),
        },
    }
}
