use std::sync::Arc;

use crate::{
    error::quaint_error_to_connector_error,
    flavour::{SqlFlavour, SqliteFlavour},
};
use migration_connector::{ConnectorError, ConnectorResult};
use quaint::{
    error::{Error as QuaintError, ErrorKind as QuaintKind},
    prelude::Queryable,
    prelude::{ConnectionInfo, Query, ResultSet},
    single::Quaint,
};
use sql_schema_describer::SqlSchema;

/// An internal mechanism for the SQL connector. It represents a single
/// connection to a database.
#[derive(Debug, Clone)]
pub(crate) struct Connection {
    quaint: Quaint,
    flavour: Arc<dyn SqlFlavour + Send + Sync + 'static>,
}

impl Connection {
    /// Establish a connection.
    pub(crate) async fn connect(connection_string: &str) -> ConnectorResult<Self> {
        let connection_info = ConnectionInfo::from_url(connection_string)
            .map_err(|err| ConnectorError::url_parse_error(err, connection_string))?;

        let flavour = crate::flavour::from_connection_info(&connection_info);

        let quaint = Quaint::new(connection_string)
            .await
            .map_err(|err| quaint_error_to_connector_error(err, &connection_info))?;

        let connection = Connection {
            flavour: flavour.into(),
            quaint,
        };

        connection.flavour.ensure_connection_validity(&connection).await?;

        Ok(connection)
    }

    /// Start an in-memory SQLite connection.
    pub(crate) async fn new_in_memory_sqlite(attached_name: &str) -> ConnectorResult<Self> {
        let quaint = quaint::single::Quaint::new_in_memory(Some(attached_name.to_owned())).map_err(|err| {
            quaint_error_to_connector_error(
                err,
                &ConnectionInfo::InMemorySqlite {
                    db_name: attached_name.to_owned(),
                },
            )
        })?;

        let flavour = SqliteFlavour {
            file_path: String::from("/dev/null"),
            attached_name: attached_name.to_owned(),
        };

        Ok(Connection {
            quaint,
            flavour: Arc::new(flavour),
        })
    }

    /// Acquire a database-level advisory lock, ensuring that no other
    /// connection is running migration engine commands on the database at the
    /// same time.
    pub(crate) async fn acquire_advisory_lock(&self) -> ConnectorResult<()> {
        self.flavour.acquire_advisory_lock(self).await
    }

    pub(crate) fn connection_info(&self) -> &ConnectionInfo {
        self.quaint.connection_info()
    }

    pub(crate) async fn describe_schema(&self) -> ConnectorResult<SqlSchema> {
        self.flavour().describe_schema(&self).await
    }

    pub(crate) async fn execute(&self, query: impl Into<Query<'_>>) -> ConnectionResult<'_, u64> {
        self.quaint
            .execute(query.into())
            .await
            .map_err(|quaint_error| ConnectionError {
                quaint_error,
                connection_info: self.connection_info(),
            })
    }

    pub(crate) fn flavour(&self) -> &(dyn SqlFlavour + Send + Sync) {
        self.flavour.as_ref()
    }

    pub(crate) fn quaint(&self) -> &Quaint {
        &self.quaint
    }

    pub(crate) async fn query(&self, query: impl Into<Query<'_>>) -> ConnectionResult<'_, ResultSet> {
        self.quaint
            .query(query.into())
            .await
            .map_err(|quaint_error| ConnectionError {
                quaint_error,
                connection_info: self.connection_info(),
            })
    }

    pub(crate) async fn query_raw(&self, sql: &str, params: &[quaint::Value<'_>]) -> ConnectionResult<'_, ResultSet> {
        self.quaint
            .query_raw(sql, params)
            .await
            .map_err(|quaint_error| ConnectionError {
                quaint_error,
                connection_info: self.connection_info(),
            })
    }

    pub(crate) async fn raw_cmd(&self, sql: &str) -> ConnectionResult<'_, ()> {
        self.quaint.raw_cmd(sql).await.map_err(|quaint_error| ConnectionError {
            quaint_error,
            connection_info: self.connection_info(),
        })
    }

    pub(crate) async fn version(&self) -> ConnectionResult<'_, Option<String>> {
        self.quaint.version().await.map_err(|quaint_error| ConnectionError {
            quaint_error,
            connection_info: self.connection_info(),
        })
    }

    /// Render a table name with the required prefixing for use with quaint query building.
    pub(crate) fn table_name<'a>(&'a self, name: &'a str) -> quaint::ast::Table<'a> {
        if self.connection_info().sql_family().is_sqlite() {
            name.into()
        } else {
            (self.connection_info().schema_name(), name).into()
        }
    }
}

#[derive(Debug)]
pub(crate) struct ConnectionError<'a> {
    quaint_error: QuaintError,
    connection_info: &'a ConnectionInfo,
}

type ConnectionResult<'a, T> = Result<T, ConnectionError<'a>>;

impl ConnectionError<'_> {
    pub(crate) fn kind(&self) -> &QuaintKind {
        self.quaint_error.kind()
    }
}

impl From<ConnectionError<'_>> for ConnectorError {
    fn from(err: ConnectionError<'_>) -> Self {
        quaint_error_to_connector_error(err.quaint_error, err.connection_info)
    }
}
