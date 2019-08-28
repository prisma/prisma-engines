//! Logic for generating Prisma data models from database introspection.
use database_introspection::*;
use datamodel::{
    common::PrismaType, dml, Datamodel, Field, FieldArity, FieldType, IdInfo, IdStrategy, Model, OnDeleteStrategy,
    RelationInfo, ScalarListStrategy,
};
use failure::Error;
use log::debug;

/// The result type.
pub type Result<T> = core::result::Result<T, Error>;

/// Calculate a data model from a database schema.
pub fn calculate_model(schema: &DatabaseSchema) -> Result<Datamodel> {
    debug!("Calculating data model");

    let mut data_model = Datamodel::new();
    for table in schema.tables.iter() {
        let mut model = Model::new(&table.name);
        for column in table.columns.iter() {
            debug!("Handling column {:?}", column);
            let field_type = calculate_field_type(&column, &table);
            let arity = match column.arity {
                ColumnArity::Required => FieldArity::Required,
                ColumnArity::Nullable => FieldArity::Optional,
                ColumnArity::List => FieldArity::List,
            };
            let id_info = calc_id_info(&column, &table);
            let scalar_list_strategy = match arity {
                FieldArity::List => Some(ScalarListStrategy::Embedded),
                _ => None,
            };
            let field = Field {
                name: column.name.clone(),
                arity,
                field_type,
                database_name: None,
                default_value: None,
                is_unique: table.is_column_unique(&column),
                id_info,
                scalar_list_strategy,
                documentation: None,
                is_generated: false,
                is_updated_at: false,
            };
            model.add_field(field);
        }
        data_model.add_model(model);
    }
    Ok(data_model)
}

fn calc_id_info(column: &Column, table: &Table) -> Option<IdInfo> {
    table.primary_key.as_ref().and_then(|pk| {
        if pk.contains_column(&column.name) {
            let strategy = match column.auto_increment {
                true => IdStrategy::Auto,
                false => IdStrategy::None,
            };
            Some(IdInfo {
                strategy,
                sequence: pk.sequence.as_ref().map(|sequence| dml::Sequence {
                    name: sequence.name.clone(),
                    allocation_size: sequence.allocation_size as i32,
                    initial_value: sequence.initial_value as i32,
                }),
            })
        } else {
            None
        }
    })
}

fn calculate_field_type(column: &Column, table: &Table) -> FieldType {
    debug!("Calculating field type for '{}'", column.name);
    // Look for a foreign key referencing this column
    match table.foreign_keys.iter().find(|fk| fk.columns.contains(&column.name)) {
        Some(fk) => {
            debug!("Found corresponding foreign key");
            let idx = fk
                .columns
                .iter()
                .position(|n| n == &column.name)
                .expect("get column FK position");
            let referenced_col = &fk.referenced_columns[idx];
            FieldType::Relation(RelationInfo {
                name: "".to_string(),
                to: fk.referenced_table.clone(),
                to_fields: vec![referenced_col.clone()],
                on_delete: match fk.on_delete_action {
                    ForeignKeyAction::Cascade => OnDeleteStrategy::Cascade,
                    _ => OnDeleteStrategy::None,
                },
            })
        }
        None => {
            debug!("Found no corresponding foreign key");
            match column.tpe.family {
                ColumnTypeFamily::Boolean => FieldType::Base(PrismaType::Boolean),
                ColumnTypeFamily::DateTime => FieldType::Base(PrismaType::DateTime),
                ColumnTypeFamily::Float => FieldType::Base(PrismaType::Float),
                ColumnTypeFamily::Int => FieldType::Base(PrismaType::Int),
                ColumnTypeFamily::String => FieldType::Base(PrismaType::String),
                // XXX: We made a conscious decision to punt on mapping of ColumnTypeFamily
                // variants that don't yet have corresponding PrismaType variants
                _ => FieldType::Base(PrismaType::String),
            }
        }
    }
}
