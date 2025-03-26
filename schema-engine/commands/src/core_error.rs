use schema_connector::ConnectorError;

/// The result type for schema engine commands
pub type CoreResult<T> = Result<T, CoreError>;

/// The top-level error type for schema engine commands
pub type CoreError = ConnectorError;
