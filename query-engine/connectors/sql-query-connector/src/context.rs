use psl::datamodel_connector::ConnectorCapabilities;
use quaint::prelude::ConnectionInfo;

pub(super) struct Context<'a> {
    connection_info: &'a ConnectionInfo,
    pub(crate) trace_id: Option<&'a str>,
    /// Maximum rows allowed at once for an insert query.
    /// None is unlimited.
    pub(crate) max_rows: Option<usize>,
    /// Maximum number of bind parameters allowed for a single query.
    /// None is unlimited.
    pub(crate) max_bind_values: Option<usize>,
    /// Capabilities supported by the loaded connector.
    ///
    /// To be used WITH CAUTION and _ONLY_ if you can't render different SQL expressions by transforming Quaint's AST when visiting it.
    /// We DO NOT want to end up with forests of ifs in the SQL connector to act differently based on each connector's capabilities.
    pub(crate) capabilities: ConnectorCapabilities,
}

impl<'a> Context<'a> {
    pub(crate) fn new(
        connection_info: &'a ConnectionInfo,
        capabilities: ConnectorCapabilities,
        trace_id: Option<&'a str>,
    ) -> Self {
        let (max_rows, default_batch_size) = match connection_info {
            ConnectionInfo::Postgres(_) => (None, 32766),
            // See https://stackoverflow.com/a/11131824/788562
            ConnectionInfo::Mysql(_) => (None, 65535),
            ConnectionInfo::Mssql(_) => (Some(1000), 2099),
            ConnectionInfo::Sqlite { .. } | ConnectionInfo::InMemorySqlite { .. } => (Some(999), 999),
        };
        Context {
            connection_info,
            trace_id,
            max_rows,
            max_bind_values: get_batch_size(default_batch_size),
            capabilities,
        }
    }

    pub(crate) fn schema_name(&self) -> &str {
        self.connection_info.schema_name()
    }
}

fn get_batch_size(default: usize) -> Option<usize> {
    use once_cell::sync::Lazy;

    /// Overrides the default number of allowed elements in query's `IN` or `NOT IN`
    /// statement for the currently loaded connector.
    /// Certain databases error out if querying with too many items. For test
    /// purposes, this value can be set with the `QUERY_BATCH_SIZE` environment
    /// value to a smaller number.
    static BATCH_SIZE_OVERRIDE: Lazy<Option<usize>> = Lazy::new(|| {
        std::env::var("QUERY_BATCH_SIZE")
            .ok()
            .map(|size| size.parse().expect("QUERY_BATCH_SIZE: not a valid size"))
    });
    (*BATCH_SIZE_OVERRIDE).or(Some(default))
}
