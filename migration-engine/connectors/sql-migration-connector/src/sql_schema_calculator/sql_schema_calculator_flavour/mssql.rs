use super::SqlSchemaCalculatorFlavour;
use crate::flavour::{MssqlFlavour, SqlFlavour};
use datamodel::{
    datamodel_connector::walker_ext_traits::DefaultValueExt,
    parser_database::{walkers::*, ScalarType},
};
use sql_schema_describer::ForeignKeyAction;

impl SqlSchemaCalculatorFlavour for MssqlFlavour {
    fn default_constraint_name(&self, default_value: DefaultValueWalker<'_>) -> Option<String> {
        Some(default_value.constraint_name(self.datamodel_connector()).into_owned())
    }

    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> serde_json::Value {
        sql_datamodel_connector::MSSQL.default_native_type_for_scalar_type(scalar_type)
    }

    fn m2m_foreign_key_action(&self, model_a: ModelWalker<'_>, model_b: ModelWalker<'_>) -> ForeignKeyAction {
        // MSSQL will crash when creating a cyclic cascade
        if model_a.name() == model_b.name() {
            ForeignKeyAction::NoAction
        } else {
            ForeignKeyAction::Cascade
        }
    }
}
