use super::SqlSchemaCalculatorFlavour;
use crate::flavour::SqliteFlavour;
use datamodel::{walkers::ScalarFieldWalker, ScalarType};
use datamodel_connector::Connector;

impl SqlSchemaCalculatorFlavour for SqliteFlavour {
    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> serde_json::Value {
        sql_datamodel_connector::SqlDatamodelConnectors::sqlite(Default::default())
            .default_native_type_for_scalar_type(scalar_type)
    }

    // Integer primary keys on SQLite are automatically assigned the rowid, which means they are automatically autoincrementing.
    fn field_is_implicit_autoincrement_primary_key(&self, field: &ScalarFieldWalker<'_>) -> bool {
        field.model().get().field_is_primary(field.name()) && field.field_type().is_int()
    }
}
