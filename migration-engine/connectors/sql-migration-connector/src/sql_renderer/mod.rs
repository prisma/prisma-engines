use crate::SqlFamily;
use sql_schema_describer::*;

mod common;
mod mysql_renderer;
mod postgres_renderer;
mod sqlite_renderer;

pub(crate) use common::IteratorJoin;
pub(crate) use mysql_renderer::{quoted as mysql_quoted, quoted_string as mysql_quoted_string};
pub(crate) use postgres_renderer::{
    quoted as postgres_quoted, quoted_string as postgres_quoted_string,
    render_column_type as postgres_render_column_type,
};
pub(crate) use sqlite_renderer::quoted as sqlite_quoted;

use crate::sql_schema_helpers::ColumnRef;
use mysql_renderer::MySqlRenderer;
use postgres_renderer::PostgresRenderer;
use sqlite_renderer::SqliteRenderer;

use std::fmt::Write as _;

pub(crate) trait SqlRenderer {
    fn write_quoted_with_schema(&self, buf: &mut String, schema: &str, name: &str) -> std::fmt::Result {
        self.write_quoted(buf, schema)?;
        write!(buf, ".")?;
        self.write_quoted(buf, name)?;

        Ok(())
    }

    fn write_quoted(&self, buf: &mut String, identifier: &str) -> std::fmt::Result;

    fn quote_with_schema(&self, schema: &str, name: &str) -> String {
        format!("{}.{}", self.quote(&schema), self.quote(&name),)
    }

    fn quote(&self, name: &str) -> String;

    fn render_column(&self, schema_name: &str, column: ColumnRef<'_>, add_fk_prefix: bool) -> String;

    fn render_references(&self, schema_name: &str, foreign_key: &ForeignKey) -> String;

    fn sql_family(&self) -> SqlFamily;
}

impl dyn SqlRenderer {
    pub fn for_family<'a>(sql_family: &SqlFamily) -> Box<dyn SqlRenderer + Send + Sync + 'a> {
        match sql_family {
            SqlFamily::Postgres => Box::new(PostgresRenderer {}),
            SqlFamily::Mysql => Box::new(MySqlRenderer {}),
            SqlFamily::Sqlite => Box::new(SqliteRenderer {}),
        }
    }
}
