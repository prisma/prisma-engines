use crate::Dedup;
use crate::SqlError;
use datamodel::IndexAlgorithm;
use datamodel::{
    common::preview_features::PreviewFeature, common::RelationNames, Datamodel, DefaultValue as DMLDef, FieldArity,
    FieldType, IndexDefinition, IndexField, Model, PrimaryKeyField, ReferentialAction, RelationField, RelationInfo,
    ScalarField, ScalarType, SortOrder, ValueGenerator as VG,
};
use introspection_connector::IntrospectionContext;
use sql_schema_describer::SQLIndexAlgorithm;
use sql_schema_describer::{
    Column, ColumnArity, ColumnTypeFamily, ForeignKey, Index, IndexType, SQLSortOrder, SqlSchema, Table,
};
use sql_schema_describer::{DefaultKind, ForeignKeyAction};
use tracing::debug;

//checks
pub fn is_old_migration_table(table: &Table) -> bool {
    table.name == "_Migration"
        && table.columns.iter().any(|c| c.name == "revision")
        && table.columns.iter().any(|c| c.name == "name")
        && table.columns.iter().any(|c| c.name == "datamodel")
        && table.columns.iter().any(|c| c.name == "status")
        && table.columns.iter().any(|c| c.name == "applied")
        && table.columns.iter().any(|c| c.name == "rolled_back")
        && table.columns.iter().any(|c| c.name == "datamodel_steps")
        && table.columns.iter().any(|c| c.name == "database_migration")
        && table.columns.iter().any(|c| c.name == "errors")
        && table.columns.iter().any(|c| c.name == "started_at")
        && table.columns.iter().any(|c| c.name == "finished_at")
}

pub fn is_new_migration_table(table: &Table) -> bool {
    table.name == "_prisma_migrations"
        && table.columns.iter().any(|c| c.name == "id")
        && table.columns.iter().any(|c| c.name == "checksum")
        && table.columns.iter().any(|c| c.name == "finished_at")
        && table.columns.iter().any(|c| c.name == "migration_name")
        && table.columns.iter().any(|c| c.name == "logs")
        && table.columns.iter().any(|c| c.name == "rolled_back_at")
        && table.columns.iter().any(|c| c.name == "started_at")
        && table.columns.iter().any(|c| c.name == "applied_steps_count")
}

pub(crate) fn is_relay_table(table: &Table) -> bool {
    table.name == "_RelayId"
        && table.columns[0].name == "id"
        && table.columns[1].name.to_lowercase() == "stablemodelidentifier"
}

pub(crate) fn is_prisma_1_or_11_list_table(table: &Table) -> bool {
    table.columns.len() == 3
        && table.columns[0].name.to_lowercase() == "nodeid"
        && table.columns[1].name == "position"
        && table.columns[2].name == "value"
}

pub(crate) fn is_prisma_1_point_1_or_2_join_table(table: &Table) -> bool {
    table.columns.len() == 2 && table.indices.len() >= 2 && common_prisma_m_to_n_relation_conditions(table)
}

pub(crate) fn is_prisma_1_point_0_join_table(table: &Table) -> bool {
    table.columns.len() == 3
        && table.indices.len() >= 2
        && table.columns.iter().any(|c| c.name == "id")
        && common_prisma_m_to_n_relation_conditions(table)
}

fn common_prisma_m_to_n_relation_conditions(table: &Table) -> bool {
    fn is_a(column: &str) -> bool {
        column.to_lowercase() == "a"
    }

    fn is_b(column: &str) -> bool {
        column.to_lowercase() == "b"
    }

    table.name.starts_with('_')
        //UNIQUE INDEX [A,B]
        && table.indices.iter().any(|i| {
            i.columns.len() == 2
                && is_a(i.columns[0].name())
                && is_b(i.columns[1].name())
                && i.is_unique()
        })
        //INDEX [B]
        && table
            .indices
            .iter()
            .any(|i| i.columns.len() == 1 && is_b(i.columns[0].name()) && i.tpe == IndexType::Normal)

        // 2 FKs
        && table.foreign_keys.len() == 2
        // Lexicographically lower model referenced by A
        && if table.foreign_keys[0].referenced_table <= table.foreign_keys[1].referenced_table {
            is_a(&table.foreign_keys[0].columns[0]) && is_b(&table.foreign_keys[1].columns[0])
        } else {
            is_b(&table.foreign_keys[0].columns[0]) && is_a(&table.foreign_keys[1].columns[0])
        }
}

//calculators

pub fn calculate_many_to_many_field(
    opposite_foreign_key: &ForeignKey,
    relation_name: String,
    is_self_relation: bool,
) -> RelationField {
    let relation_info = RelationInfo {
        name: relation_name,
        fk_name: None,
        fields: vec![],
        to: opposite_foreign_key.referenced_table.clone(),
        references: opposite_foreign_key.referenced_columns.clone(),
        on_delete: None,
        on_update: None,
    };

    let basename = opposite_foreign_key.referenced_table.clone();

    let name = match is_self_relation {
        true => format!("{}_{}", basename, opposite_foreign_key.columns[0]),
        false => basename,
    };

    RelationField::new(&name, FieldArity::List, FieldArity::List, relation_info)
}

pub(crate) fn calculate_index(index: &Index, ctx: &IntrospectionContext) -> IndexDefinition {
    debug!("Handling index  {:?}", index);
    let tpe = match index.tpe {
        IndexType::Unique => datamodel::dml::IndexType::Unique,
        IndexType::Normal => datamodel::dml::IndexType::Normal,
        IndexType::Fulltext if ctx.preview_features.contains(PreviewFeature::FullTextIndex) => {
            datamodel::dml::IndexType::Fulltext
        }
        IndexType::Fulltext => datamodel::dml::IndexType::Normal,
    };

    //We do not populate name in client by default. It increases datamodel noise,
    //and we would need to sanitize it. Users can give their own names if they want
    //and re-introspection will keep them. This is a change in introspection behaviour,
    //but due to re-introspection previous datamodels and clients should keep working as before.

    let using = if ctx.preview_features.contains(PreviewFeature::ExtendedIndexes) {
        index.algorithm.map(|algo| match algo {
            SQLIndexAlgorithm::BTree => IndexAlgorithm::BTree,
            SQLIndexAlgorithm::Hash => IndexAlgorithm::Hash,
        })
    } else {
        None
    };

    IndexDefinition {
        name: None,
        db_name: Some(index.name.clone()),
        fields: index
            .columns
            .iter()
            .map(|c| {
                let (sort_order, length) = if !ctx.preview_features.contains(PreviewFeature::ExtendedIndexes) {
                    (None, None)
                } else {
                    let sort_order = c.sort_order.map(|sort| match sort {
                        SQLSortOrder::Asc => SortOrder::Asc,
                        SQLSortOrder::Desc => SortOrder::Desc,
                    });
                    (sort_order, c.length)
                };
                IndexField {
                    name: c.name().to_string(),
                    sort_order,
                    length,
                }
            })
            .collect(),
        tpe,
        defined_on_field: index.columns.len() == 1,
        algorithm: using,
    }
}

pub(crate) fn calculate_scalar_field(table: &Table, column: &Column, ctx: &IntrospectionContext) -> ScalarField {
    debug!("Handling column {:?}", column);

    let field_type = calculate_scalar_field_type_with_native_types(column, ctx);

    let is_id = is_id(column, table);
    let arity = match column.tpe.arity {
        _ if is_id && column.auto_increment => FieldArity::Required,
        ColumnArity::Required => FieldArity::Required,
        ColumnArity::Nullable => FieldArity::Optional,
        ColumnArity::List => FieldArity::List,
    };

    let default_value = calculate_default(table, column, &arity);

    ScalarField {
        name: column.name.clone(),
        arity,
        field_type,
        database_name: None,
        default_value,
        documentation: None,
        is_generated: false,
        is_updated_at: false,
        is_commented_out: false,
        is_ignored: false,
        comment_value: None,
    }
}

pub(crate) fn calculate_relation_field(
    schema: &SqlSchema,
    table: &Table,
    foreign_key: &ForeignKey,
    m2m_table_names: &[String],
) -> Result<RelationField, SqlError> {
    debug!("Handling foreign key  {:?}", foreign_key);

    let map_action = |action: ForeignKeyAction| match action {
        ForeignKeyAction::NoAction => ReferentialAction::NoAction,
        ForeignKeyAction::Restrict => ReferentialAction::Restrict,
        ForeignKeyAction::Cascade => ReferentialAction::Cascade,
        ForeignKeyAction::SetNull => ReferentialAction::SetNull,
        ForeignKeyAction::SetDefault => ReferentialAction::SetDefault,
    };

    let relation_info = RelationInfo {
        name: calculate_relation_name(schema, foreign_key, table, m2m_table_names)?,
        fk_name: foreign_key.constraint_name.clone(),
        fields: foreign_key.columns.clone(),
        to: foreign_key.referenced_table.clone(),
        references: foreign_key.referenced_columns.clone(),
        on_delete: Some(map_action(foreign_key.on_delete_action)),
        on_update: Some(map_action(foreign_key.on_update_action)),
    };

    let columns: Vec<&Column> = foreign_key
        .columns
        .iter()
        .map(|c| table.columns.iter().find(|tc| tc.name == *c).unwrap())
        .collect();

    let arity = match columns.iter().any(|c| !c.is_required()) {
        true => FieldArity::Optional,
        false => FieldArity::Required,
    };

    let calculated_arity = match columns.iter().any(|c| c.is_required()) {
        true => FieldArity::Required,
        false => arity,
    };

    Ok(RelationField::new(
        &foreign_key.referenced_table,
        arity,
        calculated_arity,
        relation_info,
    ))
}

pub(crate) fn calculate_backrelation_field(
    schema: &SqlSchema,
    model: &Model,
    other_model: &Model,
    relation_field: &RelationField,
    relation_info: &RelationInfo,
) -> Result<RelationField, SqlError> {
    match schema.table(&model.name) {
        Err(table_name) => Err(SqlError::SchemaInconsistent {
            explanation: format!("Table {} not found.", table_name),
        }),
        Ok(table) => {
            let new_relation_info = RelationInfo {
                name: relation_info.name.clone(),
                fk_name: None,
                to: model.name.clone(),
                fields: vec![],
                references: vec![],
                on_delete: None,
                on_update: None,
            };

            // unique or id
            let other_is_unique = table.indices.iter().any(|i| {
                columns_match(
                    &i.columns.iter().map(|c| c.name().to_string()).collect::<Vec<_>>(),
                    &relation_info.fields,
                ) && i.is_unique()
            }) || columns_match(
                &table
                    .primary_key_columns()
                    .map(|c| c.name().to_string())
                    .collect::<Vec<_>>(),
                &relation_info.fields,
            );

            let arity = match relation_field.arity {
                FieldArity::Required | FieldArity::Optional if other_is_unique => FieldArity::Optional,
                FieldArity::Required | FieldArity::Optional => FieldArity::List,
                FieldArity::List => FieldArity::Optional,
            };

            //if the backrelation name would be duplicate, probably due to being a selfrelation
            let name = if model.name == other_model.name && relation_field.name == model.name {
                format!("other_{}", model.name.clone())
            } else {
                model.name.clone()
            };

            Ok(RelationField::new(&name, arity, arity, new_relation_info))
        }
    }
}

pub(crate) fn calculate_default(table: &Table, column: &Column, arity: &FieldArity) -> Option<DMLDef> {
    match (column.default.as_ref().map(|d| d.kind()), &column.tpe.family) {
        (_, _) if *arity == FieldArity::List => None,
        (_, ColumnTypeFamily::Int) if column.auto_increment => Some(DMLDef::new_expression(VG::new_autoincrement())),
        (_, ColumnTypeFamily::BigInt) if column.auto_increment => Some(DMLDef::new_expression(VG::new_autoincrement())),
        (_, ColumnTypeFamily::Int) if is_sequence(column, table) => {
            Some(DMLDef::new_expression(VG::new_autoincrement()))
        }
        (_, ColumnTypeFamily::BigInt) if is_sequence(column, table) => {
            Some(DMLDef::new_expression(VG::new_autoincrement()))
        }
        (Some(DefaultKind::Sequence(_)), _) => Some(DMLDef::new_expression(VG::new_autoincrement())),
        (Some(DefaultKind::Now), ColumnTypeFamily::DateTime) => {
            Some(set_default(DMLDef::new_expression(VG::new_now()), column))
        }
        (Some(DefaultKind::DbGenerated(default_string)), _) => Some(set_default(
            DMLDef::new_expression(VG::new_dbgenerated(default_string.clone())),
            column,
        )),
        (Some(DefaultKind::Value(val)), _) => Some(set_default(DMLDef::new_single(val.clone()), column)),
        _ => None,
    }
}

fn set_default(mut default: DMLDef, column: &Column) -> DMLDef {
    let db_name = column.default.as_ref().and_then(|df| df.constraint_name());

    if let Some(name) = db_name {
        default.set_db_name(name);
    }

    default
}

pub(crate) fn is_id(column: &Column, table: &Table) -> bool {
    table
        .primary_key
        .as_ref()
        .map(|pk| pk.is_single_primary_key(&column.name))
        .unwrap_or(false)
}

pub(crate) fn is_sequence(column: &Column, table: &Table) -> bool {
    table
        .primary_key
        .as_ref()
        .map(|pk| pk.is_single_primary_key(&column.name) && pk.sequence.is_some())
        .unwrap_or(false)
}

pub(crate) fn calculate_relation_name(
    schema: &SqlSchema,
    fk: &ForeignKey,
    table: &Table,
    m2m_table_names: &[String],
) -> Result<String, SqlError> {
    //this is not called for prisma many to many relations. for them the name is just the name of the join table.
    let referenced_model = &fk.referenced_table;
    let model_with_fk = &table.name;
    let fk_column_name = fk.columns.join("_");

    let mut fk_to_same_model: Vec<ForeignKey> = table
        .foreign_keys
        .clone()
        .into_iter()
        .filter(|fk| &fk.referenced_table == referenced_model)
        .collect();

    fk_to_same_model.clear_duplicates();

    match schema.table(referenced_model) {
        Err(table_name) => Err(SqlError::SchemaInconsistent {
            explanation: format!("Table {} not found.", table_name),
        }),
        Ok(other_table) => {
            let fk_from_other_model_to_this_exist = other_table
                .foreign_keys
                .iter()
                .any(|fk| &fk.referenced_table == model_with_fk);

            let unambiguous_name = RelationNames::name_for_unambiguous_relation(model_with_fk, referenced_model);

            // this needs to know whether there are m2m relations and then use ambiguous name path
            let name = if fk_to_same_model.len() < 2
                && !fk_from_other_model_to_this_exist
                && !m2m_table_names.contains(&unambiguous_name)
            {
                unambiguous_name
            } else {
                RelationNames::name_for_ambiguous_relation(model_with_fk, referenced_model, &fk_column_name)
            };

            Ok(name)
        }
    }
}

pub(crate) fn calculate_scalar_field_type_for_native_type(column: &Column) -> FieldType {
    debug!("Calculating field type for '{}'", column.name);
    let fdt = column.tpe.full_data_type.to_owned();

    match &column.tpe.family {
        ColumnTypeFamily::Int => FieldType::Scalar(ScalarType::Int, None, None),
        ColumnTypeFamily::BigInt => FieldType::Scalar(ScalarType::BigInt, None, None),
        ColumnTypeFamily::Float => FieldType::Scalar(ScalarType::Float, None, None),
        ColumnTypeFamily::Decimal => FieldType::Scalar(ScalarType::Decimal, None, None),
        ColumnTypeFamily::Boolean => FieldType::Scalar(ScalarType::Boolean, None, None),
        ColumnTypeFamily::String => FieldType::Scalar(ScalarType::String, None, None),
        ColumnTypeFamily::DateTime => FieldType::Scalar(ScalarType::DateTime, None, None),
        ColumnTypeFamily::Json => FieldType::Scalar(ScalarType::Json, None, None),
        ColumnTypeFamily::Uuid => FieldType::Scalar(ScalarType::String, None, None),
        ColumnTypeFamily::Binary => FieldType::Scalar(ScalarType::Bytes, None, None),
        ColumnTypeFamily::Enum(name) => FieldType::Enum(name.to_owned()),
        ColumnTypeFamily::Unsupported(_) => FieldType::Unsupported(fdt),
    }
}

pub(crate) fn calculate_scalar_field_type_with_native_types(column: &Column, ctx: &IntrospectionContext) -> FieldType {
    debug!("Calculating native field type for '{}'", column.name);
    let scalar_type = calculate_scalar_field_type_for_native_type(column);

    match scalar_type {
        FieldType::Scalar(scal_type, _, _) => match &column.tpe.native_type {
            None => scalar_type,
            Some(native_type) => {
                let native_type_instance = ctx
                    .source
                    .active_connector
                    .introspect_native_type(native_type.clone())
                    .unwrap();
                FieldType::Scalar(
                    scal_type,
                    None,
                    Some(datamodel::dml::NativeTypeInstance {
                        args: native_type_instance.args,
                        serialized_native_type: native_type_instance.serialized_native_type,
                        name: native_type_instance.name,
                    }),
                )
            }
        },
        field_type => field_type,
    }
}

// misc

pub fn deduplicate_relation_field_names(datamodel: &mut Datamodel) {
    let mut duplicated_relation_fields = vec![];

    for model in datamodel.models() {
        for field in model.relation_fields() {
            if model.fields().filter(|f| field.name == f.name()).count() > 1 {
                duplicated_relation_fields.push((
                    model.name.clone(),
                    field.name.clone(),
                    field.relation_info.name.clone(),
                ));
            }
        }
    }

    duplicated_relation_fields
        .iter()
        .for_each(|(model, field, relation_name)| {
            let mut field = datamodel.find_model_mut(model).find_relation_field_mut(field);
            //todo self vs normal relation?
            field.name = format!("{}_{}", field.name, &relation_name);
        });
}
/// Returns whether the elements of the two slices match, regardless of ordering.
pub fn columns_match(a_cols: &[String], b_cols: &[String]) -> bool {
    a_cols.len() == b_cols.len() && a_cols.iter().all(|a_col| b_cols.iter().any(|b_col| a_col == b_col))
}

pub fn replace_relation_info_field_names(target: &mut Vec<String>, old_name: &str, new_name: &str) {
    target
        .iter_mut()
        .map(|v| {
            if v == old_name {
                *v = new_name.to_string()
            }
        })
        .for_each(drop);
}

pub fn replace_pk_field_names(target: &mut Vec<PrimaryKeyField>, old_name: &str, new_name: &str) {
    target
        .iter_mut()
        .map(|field| {
            if field.name == old_name {
                field.name = new_name.to_string()
            }
        })
        .for_each(drop);
}

pub fn replace_index_field_names(target: &mut Vec<IndexField>, old_name: &str, new_name: &str) {
    target
        .iter_mut()
        .map(|field| {
            if field.name == old_name {
                field.name = new_name.to_string()
            }
        })
        .for_each(drop);
}
