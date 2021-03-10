/// The result type for migration engine commands
pub type CoreResult<T> = Result<T, migration_connector::ConnectorError>;

/// Alias for ConnectorError.
pub type CoreError = migration_connector::ConnectorError;
