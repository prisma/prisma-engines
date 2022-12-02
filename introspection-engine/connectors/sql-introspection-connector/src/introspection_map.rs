use crate::introspection_helpers as helpers;
use psl::parser_database::{self, ast};
use sql_schema_describer as sql;
use std::collections::HashMap;

/// This container is responsible for matching schema items (enums, models and tables, columns and
/// fields, foreign keys and relations...) between a SQL catalog from a database and a Prisma
/// schema.
#[derive(Default)]
pub(crate) struct IntrospectionMap {
    pub(crate) existing_enums: HashMap<sql::EnumId, ast::EnumId>,
    pub(crate) existing_models: HashMap<sql::TableId, ast::ModelId>,
    pub(crate) existing_scalar_fields: HashMap<sql::ColumnId, (ast::ModelId, ast::FieldId)>,
    pub(crate) existing_inline_relations: HashMap<sql::ForeignKeyId, parser_database::RelationId>,
    pub(crate) existing_m2m_relations: HashMap<sql::TableId, parser_database::ManyToManyRelationId>,
}

impl IntrospectionMap {
    pub(crate) fn new(sql_schema: &sql::SqlSchema, prisma_schema: &psl::ValidatedSchema) -> Self {
        let mut map = Default::default();
        match_existing_models(sql_schema, prisma_schema, &mut map);
        match_enums(sql_schema, prisma_schema, &mut map);
        match_existing_scalar_fields(sql_schema, prisma_schema, &mut map);
        match_existing_inline_relations(sql_schema, prisma_schema, &mut map);
        match_existing_m2m_relations(sql_schema, prisma_schema, &mut map);
        map
    }
}

fn match_enums(sql_schema: &sql::SqlSchema, prisma_schema: &psl::ValidatedSchema, map: &mut IntrospectionMap) {
    map.existing_enums = if prisma_schema.connector.is_provider("mysql") {
        sql_schema
            .walk_columns()
            .filter_map(|col| col.column_type_family_as_enum().map(|enm| (col, enm)))
            .filter_map(|(col, sql_enum)| {
                prisma_schema
                    .db
                    .walk_models()
                    .find(|model| model.database_name() == col.table().name())
                    .and_then(|model| model.scalar_fields().find(|sf| sf.database_name() == col.name()))
                    .and_then(|scalar_field| scalar_field.field_type_as_enum())
                    .map(|ast_enum| (sql_enum.id, ast_enum.id))
            })
            // Make sure the values are the same, otherwise we're not _really_ dealing with the same
            // enum.
            .filter(|(sql_enum_id, ast_enum_id)| {
                let sql_values = sql_schema.walk(*sql_enum_id).values();
                let prisma_values = prisma_schema.db.walk(*ast_enum_id).values();
                prisma_values.len() == sql_values.len()
                    && prisma_values.zip(sql_values).all(|(a, b)| a.database_name() == b)
            })
            .collect()
    } else {
        prisma_schema
            .db
            .walk_enums()
            .filter_map(|prisma_enum| {
                sql_schema
                    .find_enum(prisma_enum.database_name())
                    .map(|sql_id| (sql_id, prisma_enum.id))
            })
            .collect()
    }
}

fn match_existing_models(schema: &sql::SqlSchema, prisma_schema: &psl::ValidatedSchema, map: &mut IntrospectionMap) {
    map.existing_models = prisma_schema
        .db
        .walk_models()
        .filter_map(|model| {
            schema
                .find_table(model.database_name())
                .map(|sql_id| (sql_id, model.id))
        })
        .collect()
}

fn match_existing_scalar_fields(
    sql_schema: &sql::SqlSchema,
    prisma_schema: &psl::ValidatedSchema,
    map: &mut IntrospectionMap,
) {
    map.existing_scalar_fields = sql_schema
        .walk_columns()
        .filter_map(|col| {
            let model_id = map.existing_models.get(&col.table().id)?;
            let field_id = prisma_schema
                .db
                .walk(*model_id)
                .scalar_fields()
                .find(|field| field.database_name() == col.name())
                .map(|field| field.field_id())?;
            Some((col.id, (*model_id, field_id)))
        })
        .collect()
}

fn match_existing_inline_relations<'a>(
    sql_schema: &'a sql::SqlSchema,
    prisma_schema: &'a psl::ValidatedSchema,
    map: &mut IntrospectionMap,
) {
    map.existing_inline_relations = sql_schema
        .walk_foreign_keys()
        .filter_map(|fk| {
            let referencing_model = *map.existing_models.get(&fk.table().id)?;
            prisma_schema
                .db
                .walk(referencing_model)
                .relations_from()
                .filter_map(|rel| rel.refine().as_inline())
                .find(|relation| {
                    let referencing_fields = if let Some(fields) = relation.referencing_fields() {
                        fields
                    } else {
                        return false;
                    };
                    let referencing_columns = fk.constrained_columns();
                    referencing_fields.len() == referencing_columns.len()
                        && referencing_fields
                            .zip(referencing_columns)
                            .all(|(field, col)| field.database_name() == col.name())
                })
                .map(|relation| (fk.id, relation.relation_id()))
        })
        .collect()
}

fn match_existing_m2m_relations(
    sql_schema: &sql::SqlSchema,
    prisma_schema: &psl::ValidatedSchema,
    map: &mut IntrospectionMap,
) {
    map.existing_m2m_relations = sql_schema
        .table_walkers()
        .filter(|t| helpers::is_prisma_join_table(*t))
        .filter_map(|table| {
            prisma_schema
                .db
                .walk_relations()
                .filter_map(|rel| rel.refine().as_many_to_many())
                .find(|rel| rel.relation_name().to_string() == table.name()[1..])
                .map(|rel| (table.id, rel.id))
        })
        .collect()
}
