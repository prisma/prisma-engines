use crate::sql_schema_calculator::{Context, SqlSchemaCalculatorFlavour};
use psl::parser_database::walkers::*;
use sql_schema_describer::ColumnTypeFamily;

#[derive(Debug, Default)]
pub struct SurrealDbSchemaCalculatorFlavour;

impl SqlSchemaCalculatorFlavour for SurrealDbSchemaCalculatorFlavour {
    fn datamodel_connector(&self) -> &dyn psl::datamodel_connector::Connector {
        psl::builtin_connectors::SURREALDB
    }

    fn column_type_for_enum(&self, _enm: EnumWalker<'_>, _ctx: &Context<'_>) -> Option<ColumnTypeFamily> {
        Some(ColumnTypeFamily::String)
    }
}
