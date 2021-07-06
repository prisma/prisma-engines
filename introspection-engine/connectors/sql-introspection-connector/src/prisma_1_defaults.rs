use crate::warnings::{warning_default_cuid_warning, warning_default_uuid_warning, ModelAndField};
use crate::SqlFamilyTrait;
use datamodel::{dml, Datamodel, ValueGenerator};
use introspection_connector::{IntrospectionContext, Version, Warning};
use native_types::{MySqlType, PostgresType};
use sql_schema_describer::SqlSchema;

pub fn add_prisma_1_id_defaults(
    version: &Version,
    data_model: &mut Datamodel,
    schema: &SqlSchema,
    warnings: &mut Vec<Warning>,
    ctx: &IntrospectionContext,
) {
    let mut needs_to_be_changed = vec![];

    if matches!(version, Version::Prisma1 | Version::Prisma11) {
        for model in data_model.models().filter(|m| m.has_single_id_field()) {
            let id_field = model.scalar_fields().find(|f| f.is_id).unwrap();
            let table_name = model.database_name.as_ref().unwrap_or(&model.name);
            let table = schema.table(table_name).unwrap();
            let column_name = id_field.database_name.as_ref().unwrap_or(&id_field.name);
            let column = table.column(column_name).unwrap();
            let model_and_field = ModelAndField::new(&model.name, &id_field.name);

            if ctx.sql_family().is_postgres() {
                if let Some(native_type) = &column.tpe.native_type {
                    let native_type: PostgresType = serde_json::from_value(native_type.clone()).unwrap();

                    if native_type == PostgresType::VarChar(Some(25)) {
                        needs_to_be_changed.push((model_and_field, true))
                    } else if native_type == PostgresType::VarChar(Some(36)) {
                        needs_to_be_changed.push((model_and_field, false))
                    }
                }
            } else if ctx.sql_family().is_mysql() {
                if let Some(native_type) = &column.tpe.native_type {
                    let native_type: MySqlType = serde_json::from_value(native_type.clone()).unwrap();

                    if native_type == MySqlType::Char(25) {
                        needs_to_be_changed.push((model_and_field, true))
                    } else if native_type == MySqlType::Char(36) {
                        needs_to_be_changed.push((model_and_field, false))
                    }
                }
            };
        }
    }

    let mut inferred_cuids = vec![];
    let mut inferred_uuids = vec![];

    for (mf, cuid) in needs_to_be_changed {
        let field = &mut data_model.find_scalar_field_mut(&mf.model, &mf.field);
        if cuid {
            field.default_value = Some(dml::DefaultValue::Expression(ValueGenerator::new_cuid()));
            inferred_cuids.push(mf);
        } else {
            field.default_value = Some(dml::DefaultValue::Expression(ValueGenerator::new_uuid()));
            inferred_uuids.push(mf);
        }
    }

    if !inferred_cuids.is_empty() {
        warnings.push(warning_default_cuid_warning(&inferred_cuids))
    }

    if !inferred_uuids.is_empty() {
        warnings.push(warning_default_uuid_warning(&inferred_uuids))
    }
}
