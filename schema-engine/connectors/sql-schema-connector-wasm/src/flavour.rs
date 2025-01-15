mod mysql;
mod postgres;
mod sqlite;

pub(crate) trait SqlFlavour: Send + Sync + Debug {
    /// Check a connection to make sure it is usable by the schema engine.
    /// This can include some set up on the database, like ensuring that the
    /// schema we connect to exists.
    fn ensure_connection_validity(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;

    /// Return the database version as a string.
    fn version(&mut self) -> BoxFuture<'_, ConnectorResult<Option<String>>>;
}
