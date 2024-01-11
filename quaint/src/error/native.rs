use thiserror::Error;

#[derive(Debug, Error)]
pub enum NativeErrorKind {
    #[error("Error creating a database connection.")]
    ConnectionError(Box<dyn std::error::Error + Send + Sync + 'static>),

    #[error("The server terminated the connection.")]
    ConnectionClosed,

    #[error("The connection pool has been closed")]
    PoolClosed {},

    #[error(
        "Timed out fetching a connection from the pool (connection limit: {}, in use: {}, pool timeout {})",
        max_open,
        in_use,
        timeout
    )]
    PoolTimeout { max_open: u64, in_use: u64, timeout: u64 },

    #[error("Error in an I/O operation: {0}")]
    IoError(std::io::Error),

    #[error("Timed out when connecting to the database.")]
    ConnectTimeout,

    #[error("Error opening a TLS connection. {}", message)]
    TlsError { message: String },
}
