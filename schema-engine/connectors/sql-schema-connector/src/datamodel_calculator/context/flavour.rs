mod mysql;
mod postgresql;
mod sqlite;
mod sqlserver;

use sql::{ForeignKeyWalker, IndexColumnWalker, IndexWalker, TableWalker};
use sql_schema_describer as sql;

pub(super) use mysql::MysqlIntrospectionFlavour;
pub(super) use postgresql::PostgresIntrospectionFlavour;
pub(super) use sqlite::SqliteIntrospectionFlavour;
pub(super) use sqlserver::SqlServerIntrospectionFlavour;

use schema_connector::Warnings;

use super::DatamodelCalculatorContext;

pub(crate) trait IntrospectionFlavour {
    /// For columns in PostgreSQL or SQLite views, if changed in PSL,
    /// we use the changed arity instead of the always optional value from the
    /// database.
    fn keep_previous_scalar_field_arity(&self, _: sql::ColumnWalker<'_>) -> bool {
        false
    }

    fn format_view_definition(&self, definition: &str) -> String {
        let opts = sqlformat::FormatOptions {
            uppercase: true,
            ..Default::default()
        };

        sqlformat::format(definition, &Default::default(), opts)
    }

    fn generate_warnings(&self, _ctx: &DatamodelCalculatorContext<'_>, _warnings: &mut Warnings) {}

    fn uses_row_level_ttl(&self, _ctx: &DatamodelCalculatorContext<'_>, _table: TableWalker<'_>) -> bool {
        false
    }

    fn uses_non_default_index_deferring(&self, _ctx: &DatamodelCalculatorContext<'_>, _index: IndexWalker<'_>) -> bool {
        false
    }

    fn uses_non_default_foreign_key_deferring(
        &self,
        _ctx: &DatamodelCalculatorContext<'_>,
        _foreign_key: ForeignKeyWalker<'_>,
    ) -> bool {
        false
    }

    fn uses_non_default_null_position(
        &self,
        _ctx: &DatamodelCalculatorContext<'_>,
        _column: IndexColumnWalker<'_>,
    ) -> bool {
        false
    }

    fn uses_exclude_constraint(&self, _ctx: &DatamodelCalculatorContext<'_>, _table: TableWalker<'_>) -> bool {
        false
    }
}
