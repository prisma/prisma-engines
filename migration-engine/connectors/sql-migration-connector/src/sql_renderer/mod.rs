use crate::SqlFamily;
use sql_schema_describer::*;

mod common;
mod mysql_renderer;
mod postgres_renderer;
mod sqlite_renderer;

use mysql_renderer::MySqlRenderer;
use postgres_renderer::PostgresRenderer;
use sqlite_renderer::SqliteRenderer;

pub trait SqlRenderer {
    fn quote_with_schema(&self, schema: &str, name: &str) -> String {
        format!("{}.{}", self.quote(&schema), self.quote(&name),)
    }

    fn quote(&self, name: &str) -> String;

    fn render_column(&self, schema_name: &str, table: &Table, column: &Column, add_fk_prefix: bool) -> String;

    fn render_column_type(&self, t: &ColumnType) -> String;

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
