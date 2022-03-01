use super::SqlSchemaCalculatorFlavour;
use crate::flavour::SqliteFlavour;
use datamodel::{datamodel_connector::ScalarType, parser_database::walkers::*};

impl SqlSchemaCalculatorFlavour for SqliteFlavour {
    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> serde_json::Value {
        sql_datamodel_connector::SQLITE.default_native_type_for_scalar_type(scalar_type)
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
}
