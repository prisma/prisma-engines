use super::SqlSchemaCalculatorFlavour;
use crate::flavour::MssqlFlavour;
use datamodel::{walkers::ModelWalker, ScalarType};
use native_types::{MsSqlType, MsSqlTypeParameter};
use sql_schema_describer::ForeignKeyAction;

impl SqlSchemaCalculatorFlavour for MssqlFlavour {
    fn m2m_foreign_key_action(&self, model_a: &ModelWalker<'_>, model_b: &ModelWalker<'_>) -> ForeignKeyAction {
        // MSSQL will crash when creating a cyclic cascade
        if model_a.name() == model_b.name() {
            ForeignKeyAction::NoAction
        } else {
            ForeignKeyAction::Cascade
        }
    }

    fn single_field_index_name(&self, model_name: &str, field_name: &str) -> String {
        format!("{}_{}_unique", model_name, field_name)
    }

    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> serde_json::Value {
        let ty = match scalar_type {
            ScalarType::Int => MsSqlType::Int,
            ScalarType::BigInt => MsSqlType::BigInt,
            ScalarType::Float => MsSqlType::Decimal(Some((32, 16))),
            ScalarType::Decimal => MsSqlType::Decimal(Some((32, 16))),
            ScalarType::Boolean => MsSqlType::Bit,
            ScalarType::String => MsSqlType::NVarChar(Some(MsSqlTypeParameter::Number(1000))),
            ScalarType::DateTime => MsSqlType::DateTime2,
            ScalarType::Bytes => MsSqlType::VarBinary(Some(MsSqlTypeParameter::Max)),
            ScalarType::Json => MsSqlType::NVarChar(Some(MsSqlTypeParameter::Number(1000))),
        };

        serde_json::to_value(ty).expect("MsSqlType to JSON failed")
    }
}
