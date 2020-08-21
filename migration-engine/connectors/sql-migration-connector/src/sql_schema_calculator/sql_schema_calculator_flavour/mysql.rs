use super::SqlSchemaCalculatorFlavour;
use crate::{flavour::MysqlFlavour, sql_schema_calculator::SqlSchemaCalculator};
use datamodel::walkers::walk_scalar_fields;
use sql_schema_describer::{self as sql};

impl SqlSchemaCalculatorFlavour for MysqlFlavour {
    fn calculate_enums(&self, calculator: &SqlSchemaCalculator<'_>) -> Vec<sql::Enum> {
        // This is a lower bound for the size of the generated enums (we assume
        // each enum is used at least once).
        let mut enums = Vec::with_capacity(calculator.data_model.enums().len());

        let enum_fields = walk_scalar_fields(&calculator.data_model)
            .filter_map(|field| field.field_type().as_enum().map(|enum_ref| (field, enum_ref)));

        for (field, enum_tpe) in enum_fields {
            let sql_enum = sql::Enum {
                name: format!(
                    "{model_name}_{field_name}",
                    model_name = field.model().database_name(),
                    field_name = field.db_name()
                ),
                values: enum_tpe.r#enum.database_values(),
            };

            enums.push(sql_enum)
        }

        enums
    }
}
