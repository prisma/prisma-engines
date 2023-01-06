use quaint::prelude::ConnectionInfo;

pub(super) struct Context<'a> {
    connection_info: &'a ConnectionInfo,
    pub(crate) trace_id: Option<&'a str>,
}

impl<'a> Context<'a> {
    pub(crate) fn new(connection_info: &'a ConnectionInfo, trace_id: Option<&'a str>) -> Self {
        Context {
            connection_info,
            trace_id,
        }
    }

    pub(crate) fn schema_name(&self) -> &str {
        self.connection_info.schema_name()
    }
}
