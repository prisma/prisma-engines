use crate::{calculate_datamodel::CalculateDatamodelContext as Context, SqlError, SqlFamilyTrait};
use psl::{
    datamodel_connector::constraint_names::ConstraintNames,
    dml::{
        Datamodel, FieldArity, FieldType, IndexAlgorithm, IndexDefinition, IndexField, Model, OperatorClass,
        PrimaryKeyField, ReferentialAction, RelationField, RelationInfo, ScalarField, ScalarType, SortOrder,
    },
    parser_database as db, PreviewFeature, RelationNames,
};
use sql::walkers::{ColumnWalker, ForeignKeyWalker, TableWalker};
use sql_schema_describer::{
    self as sql, mssql::MssqlSchemaExt, postgres::PostgresSchemaExt, ColumnArity, ColumnTypeFamily, ForeignKeyAction,
    IndexType, SQLSortOrder, SqlSchema,
};
use std::{cmp, collections::HashSet};
use tracing::debug;

/// This function implements the reverse behaviour of the `Ord` implementation for `Option`: it
/// puts `None` values last, and otherwise orders `Some`s by their contents, like the `Ord` impl.
pub(crate) fn compare_options_none_last<T: cmp::Ord>(a: Option<T>, b: Option<T>) -> cmp::Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => cmp::Ordering::Less,
        (None, Some(_)) => cmp::Ordering::Greater,
        (None, None) => cmp::Ordering::Equal,
    }
}

pub(crate) fn is_old_migration_table(table: TableWalker<'_>) -> bool {
    table.name() == "_Migration"
        && table.columns().any(|c| c.name() == "revision")
        && table.columns().any(|c| c.name() == "name")
        && table.columns().any(|c| c.name() == "datamodel")
        && table.columns().any(|c| c.name() == "status")
        && table.columns().any(|c| c.name() == "applied")
        && table.columns().any(|c| c.name() == "rolled_back")
        && table.columns().any(|c| c.name() == "datamodel_steps")
        && table.columns().any(|c| c.name() == "database_migration")
        && table.columns().any(|c| c.name() == "errors")
        && table.columns().any(|c| c.name() == "started_at")
        && table.columns().any(|c| c.name() == "finished_at")
}

pub(crate) fn is_new_migration_table(table: TableWalker<'_>) -> bool {
    table.name() == "_prisma_migrations"
        && table.columns().any(|c| c.name() == "id")
        && table.columns().any(|c| c.name() == "checksum")
        && table.columns().any(|c| c.name() == "finished_at")
        && table.columns().any(|c| c.name() == "migration_name")
        && table.columns().any(|c| c.name() == "logs")
        && table.columns().any(|c| c.name() == "rolled_back_at")
        && table.columns().any(|c| c.name() == "started_at")
        && table.columns().any(|c| c.name() == "applied_steps_count")
}

pub(crate) fn is_relay_table(table: TableWalker<'_>) -> bool {
    table.name() == "_RelayId"
        && table.column("id").is_some()
        && table
            .columns()
            .any(|col| col.name().eq_ignore_ascii_case("stablemodelidentifier"))
}

pub(crate) fn has_created_at_and_updated_at(table: TableWalker<'_>) -> bool {
    let has_created_at = table.columns().any(|col| {
        col.name().eq_ignore_ascii_case("createdat") && col.column_type().family == ColumnTypeFamily::DateTime
    });

    let has_updated_at = table.columns().any(|col| {
        col.name().eq_ignore_ascii_case("updatedat") && col.column_type().family == ColumnTypeFamily::DateTime
    });

    has_created_at && has_updated_at
}

pub(crate) fn is_prisma_1_or_11_list_table(table: TableWalker<'_>) -> bool {
    table.columns().len() == 3
        && table.columns().any(|col| col.name().eq_ignore_ascii_case("nodeid"))
        && table.column("position").is_some()
        && table.column("value").is_some()
}

pub(crate) fn is_prisma_1_point_1_or_2_join_table(table: TableWalker<'_>) -> bool {
    table.columns().len() == 2 && table.indexes().len() >= 2 && common_prisma_m_to_n_relation_conditions(table)
}

pub(crate) fn is_prisma_1_point_0_join_table(table: TableWalker<'_>) -> bool {
    table.columns().len() == 3
        && table.indexes().len() >= 2
        && table.columns().any(|c| c.name() == "id")
        && common_prisma_m_to_n_relation_conditions(table)
}

fn common_prisma_m_to_n_relation_conditions(table: TableWalker<'_>) -> bool {
    fn is_a(column: &str) -> bool {
        column.eq_ignore_ascii_case("a")
    }

    fn is_b(column: &str) -> bool {
        column.eq_ignore_ascii_case("b")
    }

    let mut fks = table.foreign_keys();
    let first_fk = fks.next();
    let second_fk = fks.next();
    let a_b_match = || {
        let first_fk = first_fk.unwrap();
        let second_fk = second_fk.unwrap();
        let first_fk_col = first_fk.constrained_columns().next().unwrap().name();
        let second_fk_col = second_fk.constrained_columns().next().unwrap().name();
        (first_fk.referenced_table().name() <= second_fk.referenced_table().name()
            && is_a(first_fk_col)
            && is_b(second_fk_col))
            || (second_fk.referenced_table().name() <= first_fk.referenced_table().name()
                && is_b(first_fk_col)
                && is_a(second_fk_col))
    };
    table.name().starts_with('_')
        //UNIQUE INDEX [A,B]
        && table.indexes().any(|i| {
            i.columns().len() == 2
                && is_a(i.columns().next().unwrap().as_column().name())
                && is_b(i.columns().nth(1).unwrap().as_column().name())
                && i.is_unique()
        })
    //INDEX [B]
    && table
        .indexes()
        .any(|i| i.columns().len() == 1 && is_b(i.columns().next().unwrap().as_column().name()) && i.index_type() == IndexType::Normal)
        // 2 FKs
        && table.foreign_keys().len() == 2
        // Lexicographically lower model referenced by A
        && a_b_match()
}

//calculators

pub fn calculate_many_to_many_field(
    opposite_foreign_key: ForeignKeyWalker<'_>,
    relation_name: String,
    is_self_relation: bool,
) -> RelationField {
    let relation_info = RelationInfo {
        name: relation_name,
        fk_name: None,
        fields: Vec::new(),
        referenced_model: opposite_foreign_key.referenced_table_name().to_owned(),
        references: Vec::new(),
        on_delete: None,
        on_update: None,
    };

    let basename = opposite_foreign_key.referenced_table_name();

    let name = match is_self_relation {
        true => format!(
            "{}_{}",
            basename,
            opposite_foreign_key.constrained_columns().next().unwrap().name()
        ),
        false => basename.to_owned(),
    };

    RelationField::new(&name, FieldArity::List, FieldArity::List, relation_info)
}

pub(crate) fn calculate_index(index: sql::walkers::IndexWalker<'_>, ctx: &Context) -> Option<IndexDefinition> {
    let tpe = match index.index_type() {
        IndexType::Unique => psl::dml::IndexType::Unique,
        IndexType::Normal => psl::dml::IndexType::Normal,
        IndexType::Fulltext if ctx.config.preview_features().contains(PreviewFeature::FullTextIndex) => {
            psl::dml::IndexType::Fulltext
        }
        IndexType::Fulltext => psl::dml::IndexType::Normal,
        IndexType::PrimaryKey => return None,
    };

    let default_constraint_name = match index.index_type() {
        IndexType::Unique => {
            let columns = index.column_names().collect::<Vec<_>>();
            ConstraintNames::unique_index_name(index.table().name(), &columns, ctx.active_connector())
        }
        _ => {
            let columns = index.column_names().collect::<Vec<_>>();
            ConstraintNames::non_unique_index_name(index.table().name(), &columns, ctx.active_connector())
        }
    };

    let db_name = if index.name() == default_constraint_name {
        None
    } else {
        Some(index.name().to_owned())
    };

    // We do not populate name in client by default. It increases datamodel noise, and we would
    // need to sanitize it. Users can give their own names if they want and re-introspection will
    // keep them. This is a change in introspection behaviour, but due to re-introspection previous
    // datamodels and clients should keep working as before.

    Some(IndexDefinition {
        name: None,
        db_name,
        fields: index
            .columns()
            .map(|c| {
                let sort_order = c.sort_order().and_then(|sort| match sort {
                    SQLSortOrder::Asc => None,
                    SQLSortOrder::Desc => Some(SortOrder::Desc),
                });

                let operator_class = get_opclass(c.id, index.schema, ctx);

                IndexField {
                    path: vec![(c.as_column().name().to_owned(), None)],
                    sort_order,
                    length: c.length(),
                    operator_class,
                }
            })
            .collect(),
        tpe,
        defined_on_field: index.columns().len() == 1,
        algorithm: index_algorithm(index, ctx),
        clustered: index_is_clustered(index.id, index.schema, ctx),
    })
}

pub(crate) fn calculate_scalar_field(column: ColumnWalker<'_>, ctx: &Context) -> ScalarField {
    let arity = match column.column_type().arity {
        _ if column.is_single_primary_key() && column.is_autoincrement() => FieldArity::Required,
        ColumnArity::Required => FieldArity::Required,
        ColumnArity::Nullable => FieldArity::Optional,
        ColumnArity::List => FieldArity::List,
    };

    let mut default_value = crate::defaults::calculate_default(column, ctx);

    let default_default_value =
        ConstraintNames::default_name(column.table().name(), column.name(), ctx.active_connector());

    if let Some(ref mut default_value) = default_value {
        if default_value.db_name == Some(default_default_value) {
            default_value.db_name = None;
        }
    }

    ScalarField {
        name: column.name().to_owned(),
        arity,
        field_type: calculate_scalar_field_type_with_native_types(column, ctx),
        database_name: None,
        default_value,
        documentation: None,
        is_generated: false,
        is_updated_at: false,
        is_commented_out: false,
        is_ignored: false,
    }
}

pub(crate) fn calculate_relation_field(
    ctx: &Context,
    foreign_key: ForeignKeyWalker<'_>,
    m2m_table_names: &[String],
    duplicated_foreign_keys: &HashSet<sql::ForeignKeyId>,
) -> RelationField {
    debug!("Handling foreign key  {:?}", foreign_key);

    let map_action = |action: ForeignKeyAction| match action {
        ForeignKeyAction::NoAction => ReferentialAction::NoAction,
        ForeignKeyAction::Restrict => ReferentialAction::Restrict,
        ForeignKeyAction::Cascade => ReferentialAction::Cascade,
        ForeignKeyAction::SetNull => ReferentialAction::SetNull,
        ForeignKeyAction::SetDefault => ReferentialAction::SetDefault,
    };

    let cols: Vec<_> = foreign_key.constrained_columns().map(|c| c.name()).collect();
    let default_constraint_name =
        ConstraintNames::foreign_key_constraint_name(foreign_key.table().name(), &cols, ctx.active_connector());

    // destroy this when removing dml
    let fk_name = if foreign_key.constraint_name() == Some(default_constraint_name.as_str()) {
        None
    } else {
        foreign_key.constraint_name().map(String::from)
    };

    let relation_info = RelationInfo {
        name: calculate_relation_name(foreign_key, m2m_table_names, duplicated_foreign_keys),
        fk_name,
        fields: foreign_key.constrained_columns().map(|c| c.name().to_owned()).collect(),
        referenced_model: foreign_key.referenced_table().name().to_owned(),
        references: foreign_key.referenced_columns().map(|c| c.name().to_owned()).collect(),
        on_delete: None,
        on_update: None,
    };

    let arity = match foreign_key.constrained_columns().any(|c| !c.arity().is_required()) {
        true => FieldArity::Optional,
        false => FieldArity::Required,
    };

    let calculated_arity = match foreign_key.constrained_columns().any(|c| c.arity().is_required()) {
        true => FieldArity::Required,
        false => arity,
    };

    let mut rf = RelationField::new(
        foreign_key.referenced_table().name(),
        arity,
        calculated_arity,
        relation_info,
    );

    rf.supports_restrict_action(!ctx.sql_family().is_mssql());

    let on_delete = map_action(foreign_key.on_delete_action());
    let on_update = map_action(foreign_key.on_update_action());

    if rf.default_on_delete_action() != on_delete {
        rf.relation_info.on_delete = Some(on_delete);
    }

    if rf.default_on_update_action() != on_update {
        rf.relation_info.on_update = Some(on_update);
    }

    rf
}

pub(crate) fn calculate_backrelation_field(
    schema: &SqlSchema,
    model: &Model,
    other_model: &Model,
    relation_field: &RelationField,
    relation_info: &RelationInfo,
) -> Result<RelationField, SqlError> {
    match schema.table_walkers().find(|t| t.name() == model.name) {
        None => Err(SqlError::SchemaInconsistent {
            explanation: format!("Table {} not found.", &model.name),
        }),
        Some(table) => {
            let new_relation_info = RelationInfo {
                name: relation_info.name.clone(),
                fk_name: None,
                referenced_model: model.name.clone(),
                fields: vec![],
                references: vec![],
                on_delete: None,
                on_update: None,
            };

            // unique or id
            let other_is_unique = table.indexes().any(|i| {
                columns_match(
                    &i.columns()
                        .map(|c| c.as_column().name().to_string())
                        .collect::<Vec<_>>(),
                    &relation_info.fields,
                ) && i.is_unique()
            }) || columns_match(
                &table
                    .primary_key_columns()
                    .into_iter()
                    .flatten()
                    .map(|c| c.as_column().name().to_owned())
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

// This is not called for prisma many to many relations. For them the name is just the name of the join table.
fn calculate_relation_name(
    fk: ForeignKeyWalker<'_>,
    m2m_table_names: &[String],
    duplicated_foreign_keys: &HashSet<sql::ForeignKeyId>,
) -> String {
    let referenced_model = fk.referenced_table().name();
    let model_with_fk = fk.table().name();
    let fk_column_name = fk.constrained_columns().map(|c| c.name()).collect::<Vec<_>>().join("_");
    let name_is_ambiguous = {
        let mut both_ids = [fk.referenced_table().id, fk.table().id];
        both_ids.sort();
        fk.schema.walk_foreign_keys().any(|other_fk| {
            let mut other_ids = [other_fk.referenced_table().id, other_fk.table().id];
            other_ids.sort();

            other_fk.id != fk.id && both_ids == other_ids && !duplicated_foreign_keys.contains(&other_fk.id)
        })
    };

    let unambiguous_name = RelationNames::name_for_unambiguous_relation(model_with_fk, referenced_model);

    // this needs to know whether there are m2m relations and then use ambiguous name path
    if name_is_ambiguous || m2m_table_names.contains(&unambiguous_name) {
        RelationNames::name_for_ambiguous_relation(model_with_fk, referenced_model, &fk_column_name)
    } else {
        unambiguous_name
    }
}

pub(crate) fn calculate_scalar_field_type_for_native_type(column: ColumnWalker<'_>, ctx: &Context) -> FieldType {
    debug!("Calculating field type for '{}'", column.name());
    let fdt = column.column_type().full_data_type.to_owned();

    match column.column_type_family() {
        ColumnTypeFamily::Int => FieldType::Scalar(ScalarType::Int, None),
        ColumnTypeFamily::BigInt => FieldType::Scalar(ScalarType::BigInt, None),
        ColumnTypeFamily::Float => FieldType::Scalar(ScalarType::Float, None),
        ColumnTypeFamily::Decimal => FieldType::Scalar(ScalarType::Decimal, None),
        ColumnTypeFamily::Boolean => FieldType::Scalar(ScalarType::Boolean, None),
        ColumnTypeFamily::String => FieldType::Scalar(ScalarType::String, None),
        ColumnTypeFamily::DateTime => FieldType::Scalar(ScalarType::DateTime, None),
        ColumnTypeFamily::Json => FieldType::Scalar(ScalarType::Json, None),
        ColumnTypeFamily::Uuid => FieldType::Scalar(ScalarType::String, None),
        ColumnTypeFamily::Binary => FieldType::Scalar(ScalarType::Bytes, None),
        ColumnTypeFamily::Enum(id) => FieldType::Enum(ctx.enum_prisma_name(*id).to_owned()),
        ColumnTypeFamily::Unsupported(_) => FieldType::Unsupported(fdt),
    }
}

pub(crate) fn calculate_scalar_field_type_with_native_types(column: sql::ColumnWalker<'_>, ctx: &Context) -> FieldType {
    debug!("Calculating native field type for '{}'", column.name());
    let scalar_type = calculate_scalar_field_type_for_native_type(column, ctx);

    match scalar_type {
        FieldType::Scalar(scal_type, _) => match &column.column_type().native_type {
            None => scalar_type,
            Some(native_type) => {
                let is_default = ctx.active_connector().native_type_is_default_for_scalar_type(
                    native_type,
                    &dml_scalar_type_to_parser_database_scalar_type(scal_type),
                );

                if is_default {
                    FieldType::Scalar(scal_type, None)
                } else {
                    let instance = psl::dml::NativeTypeInstance::new(native_type.clone(), ctx.active_connector());

                    FieldType::Scalar(scal_type, Some(instance))
                }
            }
        },
        field_type => field_type,
    }
}

fn dml_scalar_type_to_parser_database_scalar_type(st: ScalarType) -> db::ScalarType {
    match st {
        ScalarType::Int => db::ScalarType::Int,
        ScalarType::BigInt => db::ScalarType::BigInt,
        ScalarType::Float => db::ScalarType::Float,
        ScalarType::Boolean => db::ScalarType::Boolean,
        ScalarType::String => db::ScalarType::String,
        ScalarType::DateTime => db::ScalarType::DateTime,
        ScalarType::Json => db::ScalarType::Json,
        ScalarType::Bytes => db::ScalarType::Bytes,
        ScalarType::Decimal => db::ScalarType::Decimal,
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

pub(crate) fn replace_relation_info_field_names(target: &mut Vec<String>, old_name: &str, new_name: &str) {
    for old_name in target.iter_mut().filter(|v| v.as_str() == old_name) {
        *old_name = new_name.to_owned();
    }
}

pub(crate) fn replace_pk_field_names(target: &mut Vec<PrimaryKeyField>, old_name: &str, new_name: &str) {
    for field in target.iter_mut().filter(|field| field.name == old_name) {
        field.name = new_name.to_owned();
    }
}

pub(crate) fn replace_index_field_names(target: &mut Vec<IndexField>, old_name: &str, new_name: &str) {
    let field_matches = |f: &&mut IndexField| f.path.first().map(|p| p.0.as_str()) == Some(old_name);
    for field in target.iter_mut().filter(field_matches) {
        field.path = vec![(new_name.to_string(), None)];
    }
}

fn index_algorithm(index: sql::walkers::IndexWalker<'_>, ctx: &Context) -> Option<IndexAlgorithm> {
    if !ctx.sql_family().is_postgres() {
        return None;
    }

    let data: &PostgresSchemaExt = index.schema.downcast_connector_data();

    match data.index_algorithm(index.id) {
        sql::postgres::SqlIndexAlgorithm::BTree => None,
        sql::postgres::SqlIndexAlgorithm::Hash => Some(IndexAlgorithm::Hash),
        sql::postgres::SqlIndexAlgorithm::Gist => Some(IndexAlgorithm::Gist),
        sql::postgres::SqlIndexAlgorithm::Gin => Some(IndexAlgorithm::Gin),
        sql::postgres::SqlIndexAlgorithm::SpGist => Some(IndexAlgorithm::SpGist),
        sql::postgres::SqlIndexAlgorithm::Brin => Some(IndexAlgorithm::Brin),
    }
}

fn index_is_clustered(index_id: sql::IndexId, schema: &SqlSchema, ctx: &Context) -> Option<bool> {
    if !ctx.sql_family().is_mssql() {
        return None;
    }

    let ext: &MssqlSchemaExt = schema.downcast_connector_data();
    let clustered = ext.index_is_clustered(index_id);

    if !clustered {
        return None;
    }

    Some(clustered)
}

pub(crate) fn primary_key_is_clustered(pkid: sql::IndexId, ctx: &Context) -> Option<bool> {
    if !ctx.sql_family().is_mssql() {
        return None;
    }

    let ext: &MssqlSchemaExt = ctx.schema.downcast_connector_data();

    let clustered = ext.index_is_clustered(pkid);

    if clustered {
        return None;
    }

    Some(clustered)
}

fn get_opclass(index_field_id: sql::IndexColumnId, schema: &SqlSchema, ctx: &Context) -> Option<OperatorClass> {
    if !ctx.sql_family().is_postgres() {
        return None;
    }

    let ext: &PostgresSchemaExt = schema.downcast_connector_data();

    let opclass = match ext.get_opclass(index_field_id) {
        Some(opclass) => opclass,
        None => return None,
    };

    match &opclass.kind {
        _ if opclass.is_default => None,
        sql::postgres::SQLOperatorClassKind::InetOps => Some(OperatorClass::InetOps),
        sql::postgres::SQLOperatorClassKind::JsonbOps => Some(OperatorClass::JsonbOps),
        sql::postgres::SQLOperatorClassKind::JsonbPathOps => Some(OperatorClass::JsonbPathOps),
        sql::postgres::SQLOperatorClassKind::ArrayOps => Some(OperatorClass::ArrayOps),
        sql::postgres::SQLOperatorClassKind::TextOps => Some(OperatorClass::TextOps),
        sql::postgres::SQLOperatorClassKind::BitMinMaxOps => Some(OperatorClass::BitMinMaxOps),
        sql::postgres::SQLOperatorClassKind::VarBitMinMaxOps => Some(OperatorClass::VarBitMinMaxOps),
        sql::postgres::SQLOperatorClassKind::BpcharBloomOps => Some(OperatorClass::BpcharBloomOps),
        sql::postgres::SQLOperatorClassKind::BpcharMinMaxOps => Some(OperatorClass::BpcharMinMaxOps),
        sql::postgres::SQLOperatorClassKind::ByteaBloomOps => Some(OperatorClass::ByteaBloomOps),
        sql::postgres::SQLOperatorClassKind::ByteaMinMaxOps => Some(OperatorClass::ByteaMinMaxOps),
        sql::postgres::SQLOperatorClassKind::DateBloomOps => Some(OperatorClass::DateBloomOps),
        sql::postgres::SQLOperatorClassKind::DateMinMaxOps => Some(OperatorClass::DateMinMaxOps),
        sql::postgres::SQLOperatorClassKind::DateMinMaxMultiOps => Some(OperatorClass::DateMinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::Float4BloomOps => Some(OperatorClass::Float4BloomOps),
        sql::postgres::SQLOperatorClassKind::Float4MinMaxOps => Some(OperatorClass::Float4MinMaxOps),
        sql::postgres::SQLOperatorClassKind::Float4MinMaxMultiOps => Some(OperatorClass::Float4MinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::Float8BloomOps => Some(OperatorClass::Float8BloomOps),
        sql::postgres::SQLOperatorClassKind::Float8MinMaxOps => Some(OperatorClass::Float8MinMaxOps),
        sql::postgres::SQLOperatorClassKind::Float8MinMaxMultiOps => Some(OperatorClass::Float8MinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::InetInclusionOps => Some(OperatorClass::InetInclusionOps),
        sql::postgres::SQLOperatorClassKind::InetBloomOps => Some(OperatorClass::InetBloomOps),
        sql::postgres::SQLOperatorClassKind::InetMinMaxOps => Some(OperatorClass::InetMinMaxOps),
        sql::postgres::SQLOperatorClassKind::InetMinMaxMultiOps => Some(OperatorClass::InetMinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::Int2BloomOps => Some(OperatorClass::Int2BloomOps),
        sql::postgres::SQLOperatorClassKind::Int2MinMaxOps => Some(OperatorClass::Int2MinMaxOps),
        sql::postgres::SQLOperatorClassKind::Int2MinMaxMultiOps => Some(OperatorClass::Int2MinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::Int4BloomOps => Some(OperatorClass::Int4BloomOps),
        sql::postgres::SQLOperatorClassKind::Int4MinMaxOps => Some(OperatorClass::Int4MinMaxOps),
        sql::postgres::SQLOperatorClassKind::Int4MinMaxMultiOps => Some(OperatorClass::Int4MinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::Int8BloomOps => Some(OperatorClass::Int8BloomOps),
        sql::postgres::SQLOperatorClassKind::Int8MinMaxOps => Some(OperatorClass::Int8MinMaxOps),
        sql::postgres::SQLOperatorClassKind::Int8MinMaxMultiOps => Some(OperatorClass::Int8MinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::NumericBloomOps => Some(OperatorClass::NumericBloomOps),
        sql::postgres::SQLOperatorClassKind::NumericMinMaxOps => Some(OperatorClass::NumericMinMaxOps),
        sql::postgres::SQLOperatorClassKind::NumericMinMaxMultiOps => Some(OperatorClass::NumericMinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::OidBloomOps => Some(OperatorClass::OidBloomOps),
        sql::postgres::SQLOperatorClassKind::OidMinMaxOps => Some(OperatorClass::OidMinMaxOps),
        sql::postgres::SQLOperatorClassKind::OidMinMaxMultiOps => Some(OperatorClass::OidMinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::TextBloomOps => Some(OperatorClass::TextBloomOps),
        sql::postgres::SQLOperatorClassKind::TextMinMaxOps => Some(OperatorClass::TextMinMaxOps),
        sql::postgres::SQLOperatorClassKind::TimestampBloomOps => Some(OperatorClass::TimestampBloomOps),
        sql::postgres::SQLOperatorClassKind::TimestampMinMaxOps => Some(OperatorClass::TimestampMinMaxOps),
        sql::postgres::SQLOperatorClassKind::TimestampMinMaxMultiOps => Some(OperatorClass::TimestampMinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::TimestampTzBloomOps => Some(OperatorClass::TimestampTzBloomOps),
        sql::postgres::SQLOperatorClassKind::TimestampTzMinMaxOps => Some(OperatorClass::TimestampTzMinMaxOps),
        sql::postgres::SQLOperatorClassKind::TimestampTzMinMaxMultiOps => {
            Some(OperatorClass::TimestampTzMinMaxMultiOps)
        }
        sql::postgres::SQLOperatorClassKind::TimeBloomOps => Some(OperatorClass::TimeBloomOps),
        sql::postgres::SQLOperatorClassKind::TimeMinMaxOps => Some(OperatorClass::TimeMinMaxOps),
        sql::postgres::SQLOperatorClassKind::TimeMinMaxMultiOps => Some(OperatorClass::TimeMinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::TimeTzBloomOps => Some(OperatorClass::TimeTzBloomOps),
        sql::postgres::SQLOperatorClassKind::TimeTzMinMaxOps => Some(OperatorClass::TimeTzMinMaxOps),
        sql::postgres::SQLOperatorClassKind::TimeTzMinMaxMultiOps => Some(OperatorClass::TimeTzMinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::UuidBloomOps => Some(OperatorClass::UuidBloomOps),
        sql::postgres::SQLOperatorClassKind::UuidMinMaxOps => Some(OperatorClass::UuidMinMaxOps),
        sql::postgres::SQLOperatorClassKind::UuidMinMaxMultiOps => Some(OperatorClass::UuidMinMaxMultiOps),
        sql::postgres::SQLOperatorClassKind::Raw(c) => Some(OperatorClass::Raw(c.to_string().into())),
    }
}
