//! Matching PSL and database schema information together.

mod relation_names;

use crate::{datamodel_calculator::InputContext, introspection_helpers as helpers, pair::RelationFieldDirection};
use psl::parser_database::{self, ast};
use relation_names::RelationNames;
use sql_schema_describer as sql;
use std::collections::{HashMap, HashSet};

pub(crate) use relation_names::RelationName;

/// This container is responsible for matching schema items (enums, models and tables, columns and
/// fields, foreign keys and relations...) between a SQL catalog from a database and a Prisma
/// schema.
#[derive(Default)]
pub(crate) struct IntrospectionMap<'a> {
    pub(crate) existing_enums: HashMap<sql::EnumId, ast::EnumId>,
    pub(crate) existing_models: HashMap<sql::TableId, ast::ModelId>,
    pub(crate) missing_tables_for_previous_models: HashSet<ast::ModelId>,
    pub(crate) existing_scalar_fields: HashMap<sql::ColumnId, (ast::ModelId, ast::FieldId)>,
    pub(crate) existing_inline_relations: HashMap<sql::ForeignKeyId, parser_database::RelationId>,
    pub(crate) existing_m2m_relations: HashMap<sql::TableId, parser_database::ManyToManyRelationId>,
    pub(crate) relation_names: RelationNames<'a>,
    pub(crate) inline_relation_positions: Vec<(sql::TableId, sql::ForeignKeyId, RelationFieldDirection)>,
    pub(crate) m2m_relation_positions: Vec<(sql::TableId, sql::ForeignKeyId, RelationFieldDirection)>,
}

impl<'a> IntrospectionMap<'a> {
    pub(crate) fn new(input: InputContext<'a>) -> Self {
        let sql_schema = input.schema;
        let prisma_schema = input.previous_schema;
        let mut map = Default::default();

        match_existing_models(sql_schema, prisma_schema, &mut map);
        match_enums(sql_schema, prisma_schema, &mut map);
        match_existing_scalar_fields(sql_schema, prisma_schema, &mut map);
        match_existing_inline_relations(sql_schema, prisma_schema, &mut map);
        match_existing_m2m_relations(sql_schema, prisma_schema, &mut map);
        relation_names::introspect(input, &mut map);
        position_inline_relation_fields(sql_schema, &mut map);
        position_m2m_relation_fields(sql_schema, &mut map);

        map
    }
}

/// Inlined relation fields (foreign key is defined in a model) are
/// sorted in a specific way. We handle the sorting here.
fn position_inline_relation_fields(sql_schema: &sql::SqlSchema, map: &mut IntrospectionMap) {
    for table in sql_schema
        .table_walkers()
        .filter(|t| !helpers::is_prisma_join_table(*t))
    {
        for fk in table.foreign_keys() {
            map.inline_relation_positions
                .push((fk.table().id, fk.id, RelationFieldDirection::Forward));

            map.inline_relation_positions
                .push((fk.referenced_table().id, fk.id, RelationFieldDirection::Back));
        }
    }
}

/// Many to many relation fields (foreign keys are defined in a hidden
/// join table) are sorted in a specific way. We handle the sorting
/// here.
fn position_m2m_relation_fields(sql_schema: &sql::SqlSchema, map: &mut IntrospectionMap) {
    for table in sql_schema.table_walkers().filter(|t| helpers::is_prisma_join_table(*t)) {
        let mut fks = table.foreign_keys();

        if let (Some(first_fk), Some(second_fk)) = (fks.next(), fks.next()) {
            let (fk_a, fk_b) = if first_fk
                .constrained_columns()
                .next()
                .map(|c| c.name().eq_ignore_ascii_case("a"))
                .unwrap_or(false)
            {
                (first_fk, second_fk)
            } else {
                (second_fk, first_fk)
            };

            map.m2m_relation_positions
                .push((fk_a.referenced_table().id, fk_b.id, RelationFieldDirection::Forward));

            map.m2m_relation_positions
                .push((fk_b.referenced_table().id, fk_a.id, RelationFieldDirection::Back));
        }
    }
}

/// Finding enums from the existing PSL definition, matching the
/// ones found in the database.
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

/// Finding models from the existing PSL definition, matching the
/// ones found in the database.
fn match_existing_models(schema: &sql::SqlSchema, prisma_schema: &psl::ValidatedSchema, map: &mut IntrospectionMap) {
    for model in prisma_schema.db.walk_models() {
        match schema.find_table(model.database_name()) {
            Some(sql_id) => {
                map.existing_models.insert(sql_id, model.id);
            }

            None => {
                map.missing_tables_for_previous_models.insert(model.id);
            }
        }
    }
}

/// Finding scalar fields from the existing PSL definition, matching
/// the ones found in the database.
fn match_existing_scalar_fields(
    sql_schema: &sql::SqlSchema,
    prisma_schema: &psl::ValidatedSchema,
    map: &mut IntrospectionMap,
) {
    for col in sql_schema.walk_columns() {
        let ids = map.existing_models.get(&col.table().id).and_then(|model_id| {
            let model = prisma_schema.db.walk(*model_id);

            let field = model
                .scalar_fields()
                .find(|field| field.database_name() == col.name())?;

            Some((model, field))
        });

        if let Some((model, field)) = ids {
            map.existing_scalar_fields.insert(col.id, (model.id, field.field_id()));
        }
    }
}

/// Finding inlined relations from the existing PSL definition,
/// matching the ones found in the database.
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

/// Finding many to many relations from the existing PSL definition,
/// matching the ones found in the database.
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
