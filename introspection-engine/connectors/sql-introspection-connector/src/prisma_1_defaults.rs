use crate::warnings::{warning_default_cuid_warning, warning_default_uuid_warning, ModelAndField};
use datamodel::{dml, Datamodel, ValueGenerator};
use introspection_connector::{Version, Warning};
use quaint::connector::SqlFamily;
use sql_schema_describer::SqlSchema;

const CHAR: &str = "char";
const VARCHAR: &str = "varchar";
const CHARACTER_VARYING: &str = "character varying";
const CHAR_25: &str = "char(25)";
const CHAR_36: &str = "char(36)";

pub fn add_prisma_1_id_defaults(
    family: &SqlFamily,
    version: &Version,
    data_model: &mut Datamodel,
    schema: &SqlSchema,
    warnings: &mut Vec<Warning>,
) {
    let mut needs_to_be_changed = vec![];

    match version {
        Version::Prisma1 | Version::Prisma11 => {
            for model in data_model.models().filter(|m| m.has_single_id_field()) {
                let id_field = model.scalar_fields().find(|f| f.is_id).unwrap();
                let table_name = model.database_name.as_ref().unwrap_or(&model.name);
                let table = schema.table(table_name).unwrap();
                let column_name = id_field.database_name.as_ref().unwrap_or(&id_field.name);
                let column = table.column(column_name).unwrap();
                let model_and_field = ModelAndField::new(&model.name, &id_field.name);

                match (
                    &column.tpe.data_type,
                    &column.tpe.full_data_type,
                    &column.tpe.character_maximum_length,
                    family,
                ) {
                    (dt, fdt, Some(25), SqlFamily::Postgres) if dt == CHARACTER_VARYING && fdt == VARCHAR => {
                        needs_to_be_changed.push((model_and_field, true))
                    }
                    (dt, fdt, Some(36), SqlFamily::Postgres) if dt == CHARACTER_VARYING && fdt == VARCHAR => {
                        needs_to_be_changed.push((model_and_field, false))
                    }
                    (dt, fdt, Some(25), SqlFamily::Mysql) if dt == CHAR && fdt == CHAR_25 => {
                        needs_to_be_changed.push((model_and_field, true))
                    }
                    (dt, fdt, Some(36), SqlFamily::Mysql) if dt == CHAR && fdt == CHAR_36 => {
                        needs_to_be_changed.push((model_and_field, false))
                    }
                    _ => (),
                };
            }
        }
        _ => (),
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
