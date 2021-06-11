use super::SqlSchemaCalculatorFlavour;
use crate::flavour::MssqlFlavour;
use datamodel::{
    walkers::{ModelWalker, RelationFieldWalker},
    ScalarType,
};
use datamodel_connector::Connector;
use sql_schema_describer::{self as sql, ForeignKeyAction};

impl SqlSchemaCalculatorFlavour for MssqlFlavour {
    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> serde_json::Value {
        sql_datamodel_connector::SqlDatamodelConnectors::mssql().default_native_type_for_scalar_type(scalar_type)
    }

    fn m2m_foreign_key_action(&self, model_a: &ModelWalker<'_>, model_b: &ModelWalker<'_>) -> ForeignKeyAction {
        // MSSQL will crash when creating a cyclic cascade
        if model_a.name() == model_b.name() {
            ForeignKeyAction::NoAction
        } else {
            ForeignKeyAction::Cascade
        }
    }

    fn on_delete_action(&self, rf: &RelationFieldWalker<'_>) -> sql::ForeignKeyAction {
        let action = rf
            .on_delete_action()
            .map(super::convert_referential_action)
            .unwrap_or_else(|| super::convert_referential_action(rf.default_on_delete_action()));

        if action == ForeignKeyAction::Restrict {
            ForeignKeyAction::NoAction
        } else {
            action
        }
    }

    fn single_field_index_name(&self, model_name: &str, field_name: &str) -> String {
        format!("{}_{}_unique", model_name, field_name)
    }
}
