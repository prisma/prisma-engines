use std::sync::{self, atomic::AtomicUsize};

use quaint::prelude::{ConnectionInfo, SqlFamily};
use telemetry::TraceParent;

use crate::filter::alias::Alias;

pub struct Context<'a> {
    connection_info: &'a ConnectionInfo,
    pub(crate) traceparent: Option<TraceParent>,
    /// Maximum rows allowed at once for an insert query.
    /// None is unlimited.
    pub(crate) max_insert_rows: Option<usize>,
    /// Maximum number of bind parameters allowed for a single query.
    /// None is unlimited.
    pub(crate) max_bind_values: Option<usize>,

    alias_counter: AtomicUsize,
}

impl<'a> Context<'a> {
    pub fn new(connection_info: &'a ConnectionInfo, traceparent: Option<TraceParent>) -> Self {
        let max_insert_rows = connection_info.max_insert_rows();
        let max_bind_values = connection_info.max_bind_values();

        Context {
            connection_info,
            traceparent,
            max_insert_rows,
            max_bind_values: Some(max_bind_values),

            alias_counter: Default::default(),
        }
    }

    pub fn traceparent(&self) -> Option<TraceParent> {
        self.traceparent
    }

    pub(crate) fn schema_name(&self) -> &str {
        self.connection_info.schema_name()
    }

    pub fn sql_family(&self) -> SqlFamily {
        self.connection_info.sql_family()
    }

    pub fn max_insert_rows(&self) -> Option<usize> {
        self.max_insert_rows
    }

    pub fn max_bind_values(&self) -> Option<usize> {
        self.max_bind_values
    }

    pub(crate) fn next_table_alias(&self) -> Alias {
        Alias::Table(self.alias_counter.fetch_add(1, sync::atomic::Ordering::SeqCst))
    }

    pub(crate) fn next_join_alias(&self) -> Alias {
        Alias::Join(self.alias_counter.fetch_add(1, sync::atomic::Ordering::SeqCst))
    }
}
