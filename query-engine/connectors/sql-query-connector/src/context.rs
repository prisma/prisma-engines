use quaint::prelude::ConnectionInfo;

pub(super) struct Context<'a> {
    connection_info: &'a ConnectionInfo,
    pub(crate) trace_id: Option<&'a str>,
    /// Maximum rows allowed at once for an insert query.
    /// None is unlimited.
    pub(crate) max_insert_rows: Option<usize>,
    /// Maximum number of bind parameters allowed for a single query.
    /// None is unlimited.
    pub(crate) max_bind_values: Option<usize>,
}

impl<'a> Context<'a> {
    pub(crate) fn new(connection_info: &'a ConnectionInfo, trace_id: Option<&'a str>) -> Self {
        let max_insert_rows = connection_info.max_insert_rows();
        let max_bind_values = connection_info.max_bind_values();

        Context {
            connection_info,
            trace_id,
            max_insert_rows,
            max_bind_values: Some(max_bind_values),
        }
    }

    pub(crate) fn schema_name(&self) -> &str {
        self.connection_info.schema_name()
    }
}
