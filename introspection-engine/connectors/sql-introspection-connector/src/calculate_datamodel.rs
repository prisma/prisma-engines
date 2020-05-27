use crate::commenting_out_guardrails::commenting_out_guardrails;
use crate::misc_helpers::*;
use crate::sanitize_datamodel_names::sanitize_datamodel_names;
use crate::version_checker::VersionChecker;
use crate::SqlIntrospectionResult;
use datamodel::{dml, Datamodel, FieldType, Model, ValueGenerator};
use introspection_connector::{IntrospectionResult, Version, Warning};
use quaint::connector::SqlFamily;
use sql_schema_describer::*;
use tracing::debug;

/// Calculate a data model from a database schema.
pub fn calculate_datamodel(schema: &SqlSchema, family: &SqlFamily) -> SqlIntrospectionResult<IntrospectionResult> {
    debug!("Calculating data model.");

    let mut version_check = VersionChecker::new(family.clone(), schema);

    let mut data_model = Datamodel::new();
    for table in schema
        .tables
        .iter()
        .filter(|table| !is_migration_table(&table))
        .filter(|table| !is_prisma_1_point_1_or_2_join_table(&table))
        .filter(|table| !is_prisma_1_point_0_join_table(&table))
        .filter(|table| !is_relay_table(&table))
    {
        debug!("Calculating model: {}", table.name);
        let mut model = Model::new(table.name.clone(), None);

        for column in &table.columns {
            version_check.uses_non_prisma_type(&column.tpe);
            let field = calculate_scalar_field(&table, &column);
            model.add_field(field);
        }

        let mut foreign_keys_copy = table.foreign_keys.clone();
        let model_copy = model.clone();
        foreign_keys_copy.clear_duplicates();

        for foreign_key in foreign_keys_copy.iter().filter(|fk| {
            !fk.columns
                .iter()
                .any(|c| matches!(model_copy.find_field(c).unwrap().field_type, FieldType::Unsupported(_)))
        }) {
            version_check.has_inline_relations(table);
            version_check.uses_on_delete(foreign_key, table);
            model.add_field(calculate_relation_field(schema, table, foreign_key));
        }

        for index in table
            .indices
            .iter()
            .filter(|i| !(i.columns.len() == 1 && i.is_unique()))
        {
            model.add_index(calculate_index(index));
        }

        if table.primary_key_columns().len() > 1 {
            model.id_fields = table.primary_key_columns();
        }

        version_check.always_has_created_at_updated_at(table, &model);

        data_model.add_model(model);
    }

    for e in schema.enums.iter() {
        data_model.add_enum(dml::Enum {
            name: e.name.clone(),
            values: e.values.iter().map(|v| dml::EnumValue::new(v, None)).collect(),
            database_name: None,
            documentation: None,
        });
    }

    let mut fields_to_be_added = Vec::new();

    // add backrelation fields
    for model in data_model.models.iter() {
        for relation_field in model.fields.iter() {
            if let FieldType::Relation(relation_info) = &relation_field.field_type {
                if data_model
                    .related_field(
                        &model.name,
                        &relation_info.to,
                        &relation_info.name,
                        &relation_field.name,
                    )
                    .is_none()
                {
                    let other_model = data_model.find_model(relation_info.to.as_str()).unwrap();
                    let field = calculate_backrelation_field(schema, model, other_model, relation_field, relation_info);

                    fields_to_be_added.push((other_model.name.clone(), field));
                }
            }
        }
    }

    // add prisma many to many relation fields
    for table in schema
        .tables
        .iter()
        .filter(|table| is_prisma_1_point_1_or_2_join_table(&table) || is_prisma_1_point_0_join_table(&table))
    {
        if let (Some(f), Some(s)) = (table.foreign_keys.get(0), table.foreign_keys.get(1)) {
            let is_self_relation = f.referenced_table == s.referenced_table;

            fields_to_be_added.push((
                s.referenced_table.clone(),
                calculate_many_to_many_field(f, table.name[1..].to_string(), is_self_relation),
            ));
            fields_to_be_added.push((
                f.referenced_table.clone(),
                calculate_many_to_many_field(s, table.name[1..].to_string(), is_self_relation),
            ));
        }
    }

    for (model, field) in fields_to_be_added {
        let model = data_model.find_model_mut(&model).unwrap();
        model.add_field(field);
    }

    //todo sanitizing might need to be adjusted to also change the fields in the RelationInfo
    sanitize_datamodel_names(&mut data_model);

    let mut warnings: Vec<Warning> = vec![];
    let mut commenting_out_warnings = commenting_out_guardrails(&mut data_model);
    warnings.append(commenting_out_warnings.as_mut());

    deduplicate_field_names(&mut data_model);

    let version = version_check.version(&warnings);

    //--------------------------------------------------------------------------------
    use crate::commenting_out_guardrails::ModelAndField;

    //fetch character maximum length for postgres https://www.postgresql.org/docs/9.5/infoschema-columns.html

    let mut needs_to_be_changed = vec![];

    let varchar = "varchar".to_string();
    let character_varying = "character varying".to_string();
    let varchar_25 = "varchar(25)".to_string();
    let varchar_36 = "varchar(36)".to_string();
    match version {
        _ => {
            //todo  Version::Prisma1 | Version::Prisma11 => {
            for model in &data_model.models {
                let id_field = model.fields.iter().find(|f| f.is_id).unwrap();
                let table = schema.table(&model.name).unwrap();
                let column = table
                    .column(id_field.database_name.as_ref().unwrap_or(&id_field.name))
                    .unwrap();

                let model_and_field = ModelAndField {
                    model: model.name.clone(),
                    field: id_field.name.clone(),
                };

                println!("{}", &column.tpe.data_type);
                println!("{}", &column.tpe.full_data_type);
                match (
                    &column.tpe.data_type,
                    &column.tpe.full_data_type,
                    &column.tpe.character_maximum_length,
                    family,
                ) {
                    (dt, fdt, Some(25), SqlFamily::Postgres) if *dt == character_varying && *fdt == varchar => {
                        needs_to_be_changed.push((model_and_field, true))
                    }
                    (dt, fdt, Some(36), SqlFamily::Postgres) if *dt == character_varying && *fdt == varchar => {
                        needs_to_be_changed.push((model_and_field, false))
                    }
                    (dt, fdt, Some(25), SqlFamily::Mysql) if *dt == varchar && *fdt == varchar_25 => {
                        needs_to_be_changed.push((model_and_field, true))
                    }
                    (dt, fdt, Some(36), SqlFamily::Mysql) if *dt == varchar && *fdt == varchar_36 => {
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
        let field = &mut data_model
            .find_model_mut(&mf.model)
            .unwrap()
            .find_field_mut(&mf.field)
            .unwrap();
        if cuid {
            field.default_value = Some(dml::DefaultValue::Expression(ValueGenerator::new_cuid()));
            inferred_cuids.push(mf);
        } else {
            field.default_value = Some(dml::DefaultValue::Expression(ValueGenerator::new_uuid()));
            inferred_uuids.push(mf);
        }
    }

    if !inferred_cuids.is_empty() {
        warnings.push(Warning {
            code: 5,
            message:
                "These id fields had a `@default(cuid())` added because we believe the schema was created by Prisma 1."
                    .into(),
            affected: serde_json::to_value(&inferred_cuids).unwrap(),
        })
    }

    if !inferred_uuids.is_empty() {
        warnings.push(Warning {
            code: 6,
            message:
                "These id fields had a `@default(uuid())` added because we believe the schema was created by Prisma 1."
                    .into(),
            affected: serde_json::to_value(&inferred_uuids).unwrap(),
        })
    }
    //--------------------------------------------------------------------

    debug!("Done calculating data model {:?}", data_model);
    Ok(IntrospectionResult {
        datamodel: data_model,
        version,
        warnings,
    })
}

trait Dedup<T: PartialEq + Clone> {
    fn clear_duplicates(&mut self);
}

impl<T: PartialEq + Clone> Dedup<T> for Vec<T> {
    fn clear_duplicates(&mut self) {
        let mut already_seen = vec![];
        self.retain(|item| match already_seen.contains(item) {
            true => false,
            _ => {
                already_seen.push(item.clone());
                true
            }
        })
    }
}
