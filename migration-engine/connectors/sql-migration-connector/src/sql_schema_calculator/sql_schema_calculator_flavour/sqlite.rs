use super::SqlSchemaCalculatorFlavour;
use crate::flavour::{SqlFlavour, SqliteFlavour};
use datamodel::{walkers::ScalarFieldWalker, ScalarType};
use datamodel_connector::Connector;
use migration_connector::MigrationFeature;

impl SqlSchemaCalculatorFlavour for SqliteFlavour {
    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> serde_json::Value {
        sql_datamodel_connector::SqlDatamodelConnectors::sqlite()
            .default_native_type_for_scalar_type(scalar_type, self.features().contains(MigrationFeature::NativeTypes))
    }

    // Integer primary keys on SQLite are automatically assigned the rowid, which means they are automatically autoincrementing.
    fn field_is_implicit_autoincrement_primary_key(&self, field: &ScalarFieldWalker<'_>) -> bool {
        field.is_id() && field.field_type().is_int()
    }
}
