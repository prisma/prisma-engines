use crate::commenting_out_guardrails::commenting_out_guardrails;
use crate::misc_helpers::*;
use crate::misc_helpers::{is_prisma_1_or_11_list_table, is_relay_table};
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

    let migration_table = schema.tables.iter().any(|table| is_migration_table(&table));
    let has_prisma_1_join_table = schema.tables.iter().any(|table| is_prisma_1_point_0_join_table(&table));
    let has_prisma_1_1_or_2_join_table = schema
        .tables
        .iter()
        .any(|table| is_prisma_1_point_1_or_2_join_table(&table));
    let mut uses_on_delete = false;
    let mut always_has_created_at_updated_at = true;
    let mut uses_non_prisma_types = false;

    let sqlite_types = vec![
        ("BOOLEAN", "BOOLEAN"),
        ("DATE", "DATE"),
        ("REAL", "REAL"),
        ("INTEGER", "INTEGER"),
        ("TEXT", "TEXT"),
    ];
    let postgres_types = vec![
        ("boolean", "bool"),
        ("timestamp without time zone", "timestamp"),
        ("numeric", "numeric"),
        ("integer", "int4"),
        ("text", "text"),
    ];
    let mysql_types = vec![
        ("tinyint", "tinyint(1)"),
        ("datetime", "datetime(3)"),
        ("decimal", "decimal(65,30)"),
        ("int", "int"),
        ("int", "int(11)"),
        ("varchar", "varchar(191)"),
        ("char", "char(25)"),
        ("char", "char(36)"),
        ("varchar", "varchar(25)"),
        ("varchar", "varchar(36)"),
        ("text", "text"),
        ("mediumtext", "mediumtext"),
        ("int", "int(4)"),
    ];

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
            println!("DT: {}, FDT: {}", &column.tpe.data_type, &column.tpe.full_data_type);
            match (&column.tpe.data_type, &column.tpe.full_data_type, family) {
                (dt, fdt, SqlFamily::Postgres) if !postgres_types.contains(&(dt, fdt)) => uses_non_prisma_types = true,
                (dt, fdt, SqlFamily::Mysql) if !mysql_types.contains(&(dt, fdt)) => uses_non_prisma_types = true,
                (dt, fdt, SqlFamily::Sqlite) if !sqlite_types.contains(&(dt, fdt)) => uses_non_prisma_types = true,
                _ => (),
            };

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
            println!("{:?}", foreign_key);
            if foreign_key.on_delete_action != ForeignKeyAction::SetNull {
                if !is_prisma_1_or_11_list_table(table) && foreign_key.on_delete_action != ForeignKeyAction::Cascade {
                    uses_on_delete = true
                }
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

        if !is_prisma_1_or_11_list_table(table) && !is_relay_table(table) && !model.has_created_at_and_updated_at() {
            println!("Who am I: {}", table.name);
            always_has_created_at_updated_at = false
        }

        println!("{:?}", model);
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

    println!("MigrationTable: {}", migration_table);
    println!("UsesOnDelete: {}", uses_on_delete);
    println!("UsesNonPrismaTypes: {}", uses_non_prisma_types);
    println!("AlwaysCreatedAtUpdatedAt: {}", always_has_created_at_updated_at);
    println!("Prisma11Or2JoinTable: {}", has_prisma_1_1_or_2_join_table);

    let version = match family {
        SqlFamily::Sqlite if migration_table && !uses_on_delete && !uses_non_prisma_types => Version::Prisma2,
        SqlFamily::Sqlite => Version::NonPrisma,
        SqlFamily::Mysql if migration_table && !uses_on_delete && !uses_non_prisma_types => Version::Prisma2,
        SqlFamily::Mysql
            if !migration_table
                && !uses_on_delete
                && !uses_non_prisma_types
                && always_has_created_at_updated_at
                && !has_prisma_1_1_or_2_join_table =>
        {
            Version::Prisma1
        }
        SqlFamily::Mysql
            if !migration_table && !uses_on_delete && !uses_non_prisma_types && !has_prisma_1_join_table =>
        {
            Version::Prisma11
        }
        SqlFamily::Mysql => Version::NonPrisma,
        SqlFamily::Postgres
            if !migration_table
                && !uses_on_delete
                && !uses_non_prisma_types
                && always_has_created_at_updated_at
                && !has_prisma_1_join_table =>
        {
            Version::Prisma1
        }
        SqlFamily::Postgres if migration_table && !uses_on_delete && !uses_non_prisma_types => Version::Prisma2,
        SqlFamily::Postgres
            if !migration_table && !uses_on_delete && !uses_non_prisma_types && !has_prisma_1_1_or_2_join_table =>
        {
            Version::Prisma11
        }
        SqlFamily::Postgres => Version::NonPrisma,
    };

    println!("VERSION: {:?}", version);
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
