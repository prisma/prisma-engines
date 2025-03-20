use crate::sql_schema_calculator::{Context, SqlSchemaCalculatorFlavour};
use psl::parser_database::{walkers::*, ScalarType};
use sql_schema_describer::ColumnTypeFamily;

#[derive(Debug, Default)]
pub struct SqliteSchemaCalculatorFlavour;

impl SqlSchemaCalculatorFlavour for SqliteSchemaCalculatorFlavour {
    fn datamodel_connector(&self) -> &dyn psl::datamodel_connector::Connector {
        psl::builtin_connectors::SQLITE
    }

    // Integer primary keys on SQLite are automatically assigned the rowid, which means they are automatically autoincrementing.
    fn field_is_implicit_autoincrement_primary_key(&self, field: ScalarFieldWalker<'_>) -> bool {
        field
            .model()
            .primary_key()
            .map(|pk| pk.contains_exactly_fields([field].into_iter()))
            .unwrap_or(false)
            && field.scalar_type() == Some(ScalarType::Int)
    }

    fn column_type_for_enum(&self, _enm: EnumWalker<'_>, _ctx: &Context<'_>) -> Option<ColumnTypeFamily> {
        Some(ColumnTypeFamily::String)
    }
}
