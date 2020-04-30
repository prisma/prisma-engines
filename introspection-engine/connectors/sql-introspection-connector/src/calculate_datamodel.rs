use crate::commenting_out_guardrails::commenting_out_guardrails;
use crate::misc_helpers::*;
use crate::sanitize_datamodel_names::sanitize_datamodel_names;
use crate::SqlIntrospectionResult;
use datamodel::{dml, Datamodel, FieldType, Model};
use introspection_connector::{IntrospectionResult, Version};
use quaint::connector::SqlFamily;
use sql_schema_describer::*;
use tracing::debug;

/// Calculate a data model from a database schema.
pub fn calculate_datamodel(schema: &SqlSchema, family: &SqlFamily) -> SqlIntrospectionResult<IntrospectionResult> {
    debug!("Calculating data model.");

    let has_migration_table = schema.tables.iter().any(|table| is_migration_table(&table));
    let has_prisma_1_join_table = schema.tables.iter().any(|table| is_prisma_1_point_0_join_table(&table));
    let has_prisma_1_1_or_2_join_table = schema
        .tables
        .iter()
        .any(|table| is_prisma_1_point_1_or_2_join_table(&table));
    let mut uses_on_delete = false;
    let mut always_has_created_at_updated_at = false;
    let mut uses_non_prisma_types = false; // should check all scalar fields and needs mapping SQLFamily -> Types

    //Currently from Migration Engine
    //Types positive list, complicated by enums -.-

    // SQLITE   Types
    // "BOOLEAN","DATE","REAL","INTEGER","TEXT"
    // POSTGRES Types
    // Array types are only a P2 thing on Postgres
    // "boolean", "timestamp(3)", "Decimal(65,30)", "integer", "text"
    // native enums are only a P2 thing "ENUM_NAME"
    // MYSQL    Types
    // "boolean","datetime(3)", "Decimal(65,30)", "int", "varchar()",
    // native enums are only a P2 thing "ENUM()"

    let mut data_model = Datamodel::new();
    for table in schema
        .tables
        .iter()
        .filter(|table| !is_migration_table(&table))
        .filter(|table| !is_prisma_1_point_1_or_2_join_table(&table))
        .filter(|table| !is_prisma_1_point_0_join_table(&table))
    {
        debug!("Calculating model: {}", table.name);
        let mut model = Model::new(table.name.clone(), None);

        for column in &table.columns {
            // todo check actually used columntypes here
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
            if foreign_key.on_delete_action != ForeignKeyAction::SetNull {
                uses_on_delete = true
            }
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

        if !model.has_created_at_and_updated_at() {
            always_has_created_at_updated_at = false
        }

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
    let warnings = commenting_out_guardrails(&mut data_model);

    deduplicate_field_names(&mut data_model);

    debug!("Done calculating data model {:?}", data_model);

    let version = match family {
        SqlFamily::Sqlite if has_migration_table && !uses_on_delete && !uses_non_prisma_types => Version::Prisma2,
        SqlFamily::Sqlite => Version::NonPrisma,
        SqlFamily::Mysql if has_migration_table && !uses_on_delete && !uses_non_prisma_types => Version::Prisma2,
        SqlFamily::Mysql
            if !has_migration_table
                && !uses_on_delete
                && !uses_non_prisma_types
                && always_has_created_at_updated_at
                && !has_prisma_1_1_or_2_join_table =>
        {
            Version::Prisma1
        }
        SqlFamily::Mysql
            if !has_migration_table
                && !uses_on_delete
                && !uses_non_prisma_types
                && always_has_created_at_updated_at
                && !has_prisma_1_join_table =>
        {
            Version::Prisma11
        }
        SqlFamily::Mysql => Version::NonPrisma,
        SqlFamily::Postgres if has_migration_table && !uses_on_delete && !uses_non_prisma_types => Version::Prisma2,
        SqlFamily::Postgres
            if !has_migration_table
                && !uses_on_delete
                && !uses_non_prisma_types
                && always_has_created_at_updated_at
                && !has_prisma_1_1_or_2_join_table =>
        {
            Version::Prisma1
        }
        SqlFamily::Postgres
            if !has_migration_table
                && !uses_on_delete
                && !uses_non_prisma_types
                && always_has_created_at_updated_at
                && !has_prisma_1_join_table =>
        {
            Version::Prisma11
        }
        SqlFamily::Postgres => Version::NonPrisma,
    };

    Ok(IntrospectionResult {
        datamodel: data_model,
        warnings,
        version,
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
