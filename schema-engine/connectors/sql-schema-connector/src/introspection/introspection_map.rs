//! Matching PSL and database schema information together.

mod relation_names;

use crate::introspection::{
    datamodel_calculator::DatamodelCalculatorContext, introspection_helpers as helpers,
    introspection_pair::RelationFieldDirection, sanitize_datamodel_names,
};
use psl::{
    parser_database::{self as db, ScalarFieldId},
    PreviewFeature,
};
use relation_names::RelationNames;
use sql_schema_describer as sql;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

pub(crate) use relation_names::RelationName;

/// This container is responsible for matching schema items (enums, models and tables, columns and
/// fields, foreign keys and relations...) between a SQL catalog from a database and a Prisma
/// schema.
#[derive(Default)]
pub(crate) struct IntrospectionMap<'a> {
    pub(crate) existing_enums: HashMap<sql::EnumId, db::EnumId>,
    pub(crate) existing_models: HashMap<sql::TableId, db::ModelId>,
    pub(crate) existing_views: HashMap<sql::ViewId, db::ModelId>,
    pub(crate) missing_tables_for_previous_models: HashSet<db::ModelId>,
    pub(crate) missing_views_for_previous_models: HashSet<db::ModelId>,
    pub(crate) existing_model_scalar_fields: HashMap<sql::TableColumnId, ScalarFieldId>,
    pub(crate) existing_view_scalar_fields: HashMap<sql::ViewColumnId, ScalarFieldId>,
    pub(crate) existing_inline_relations: HashMap<sql::ForeignKeyId, db::RelationId>,
    pub(crate) existing_m2m_relations: HashMap<sql::TableId, db::ManyToManyRelationId>,
    pub(crate) relation_names: RelationNames<'a>,
    pub(crate) inline_relation_positions: Vec<(sql::TableId, sql::ForeignKeyId, RelationFieldDirection)>,
    pub(crate) m2m_relation_positions: Vec<(sql::TableId, sql::ForeignKeyId, RelationFieldDirection)>,
    pub(crate) top_level_names: HashMap<Cow<'a, str>, usize>,
}

impl<'a> IntrospectionMap<'a> {
    pub(crate) fn new(ctx: &DatamodelCalculatorContext<'a>) -> Self {
        let sql_schema = ctx.sql_schema;
        let prisma_schema = ctx.previous_schema;
        let mut map = Default::default();

        match_existing_models(sql_schema, prisma_schema, &mut map);
        match_existing_views(sql_schema, prisma_schema, &mut map);
        match_enums(sql_schema, prisma_schema, &mut map);
        match_existing_scalar_fields(sql_schema, prisma_schema, &mut map);
        match_existing_inline_relations(sql_schema, prisma_schema, &mut map);
        match_existing_m2m_relations(sql_schema, prisma_schema, ctx, &mut map);
        relation_names::introspect(ctx, &mut map);
        position_inline_relation_fields(sql_schema, ctx, &mut map);
        position_m2m_relation_fields(sql_schema, ctx, &mut map);
        populate_top_level_names(sql_schema, prisma_schema, ctx, &mut map);

        map
    }
}

fn populate_top_level_names<'a>(
    sql_schema: &'a sql::SqlSchema,
    prisma_schema: &'a psl::ValidatedSchema,
    ctx: &DatamodelCalculatorContext<'_>,
    map: &mut IntrospectionMap<'a>,
) {
    for table in sql_schema
        .table_walkers()
        .filter(|t| !helpers::is_prisma_m_to_n_relation(*t, ctx.flavour.uses_pk_in_m2m_join_tables(ctx)))
    {
        let name = map
            .existing_models
            .get(&table.id)
            .map(|id| prisma_schema.db.walk(*id))
            .map(|m| Cow::Borrowed(m.name()))
            .unwrap_or_else(|| sanitize_datamodel_names::sanitize_string(table.name()));

        let count = map.top_level_names.entry(name).or_default();
        *count += 1;
    }

    for r#enum in sql_schema.enum_walkers() {
        let name = map
            .existing_enums
            .get(&r#enum.id)
            .map(|id| prisma_schema.db.walk(*id))
            .map(|m| Cow::Borrowed(m.name()))
            .unwrap_or_else(|| sanitize_datamodel_names::sanitize_string(r#enum.name()));

        let count = map.top_level_names.entry(name).or_default();
        *count += 1;
    }

    // we do not want dupe warnings for models clashing with views,
    // if not using the preview.
    if prisma_schema
        .configuration
        .preview_features()
        .contains(PreviewFeature::Views)
    {
        for view in sql_schema.view_walkers() {
            let name = map
                .existing_views
                .get(&view.id)
                .map(|id| prisma_schema.db.walk(*id))
                .map(|m| Cow::Borrowed(m.name()))
                .unwrap_or_else(|| sanitize_datamodel_names::sanitize_string(view.name()));

            let count = map.top_level_names.entry(name).or_default();
            *count += 1;
        }
    }
}

/// Inlined relation fields (foreign key is defined in a model) are
/// sorted in a specific way. We handle the sorting here.
fn position_inline_relation_fields(
    sql_schema: &sql::SqlSchema,
    ctx: &DatamodelCalculatorContext<'_>,
    map: &mut IntrospectionMap<'_>,
) {
    for table in sql_schema
        .table_walkers()
        .filter(|t| !helpers::is_prisma_m_to_n_relation(*t, ctx.flavour.uses_pk_in_m2m_join_tables(ctx)))
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
fn position_m2m_relation_fields(
    sql_schema: &sql::SqlSchema,
    ctx: &DatamodelCalculatorContext<'_>,
    map: &mut IntrospectionMap<'_>,
) {
    for table in sql_schema
        .table_walkers()
        .filter(|t| helpers::is_prisma_m_to_n_relation(*t, ctx.flavour.uses_pk_in_m2m_join_tables(ctx)))
    {
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
fn match_enums(sql_schema: &sql::SqlSchema, prisma_schema: &psl::ValidatedSchema, map: &mut IntrospectionMap<'_>) {
    map.existing_enums = if prisma_schema.connector.is_provider("mysql") {
        sql_schema
            .walk_table_columns()
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
                    .find_enum(prisma_enum.database_name(), prisma_enum.schema().map(|s| s.0))
                    .map(|sql_id| (sql_id, prisma_enum.id))
            })
            .collect()
    }
}

/// Finding models from the existing PSL definition, matching the
/// ones found in the database.
fn match_existing_models(
    schema: &sql::SqlSchema,
    prisma_schema: &psl::ValidatedSchema,
    map: &mut IntrospectionMap<'_>,
) {
    for model in prisma_schema.db.walk_models() {
        match schema.find_table(model.database_name(), model.schema_name()) {
            Some(sql_id) => {
                map.existing_models.insert(sql_id, model.id);
            }

            None => {
                map.missing_tables_for_previous_models.insert(model.id);
            }
        }
    }
}

/// Finding views from the existing PSL definition, matching the
/// ones found in the database.
fn match_existing_views(
    sql_schema: &sql::SqlSchema,
    prisma_schema: &psl::ValidatedSchema,
    map: &mut IntrospectionMap<'_>,
) {
    for view in prisma_schema.db.walk_views() {
        match sql_schema.find_view(view.database_name(), view.schema_name()) {
            Some(sql_id) => {
                map.existing_views.insert(sql_id, view.id);
            }

            None => {
                map.missing_views_for_previous_models.insert(view.id);
            }
        }
    }
}

/// Finding scalar fields from the existing PSL definition, matching
/// the ones found in the database.
fn match_existing_scalar_fields(
    sql_schema: &sql::SqlSchema,
    prisma_schema: &psl::ValidatedSchema,
    map: &mut IntrospectionMap<'_>,
) {
    for col in sql_schema.walk_table_columns() {
        let field = map.existing_models.get(&col.table().id).and_then(|model_id| {
            prisma_schema
                .db
                .walk(*model_id)
                .scalar_fields()
                .find(|field| field.database_name() == col.name())
        });

        if let Some(field) = field {
            map.existing_model_scalar_fields.insert(col.id, field.id);
        }
    }

    for col in sql_schema.walk_view_columns() {
        let field = map.existing_views.get(&col.view().id).and_then(|view_id| {
            prisma_schema
                .db
                .walk(*view_id)
                .scalar_fields()
                .find(|field| field.database_name() == col.name())
        });

        if let Some(field) = field {
            map.existing_view_scalar_fields.insert(col.id, field.id);
        }
    }
}

/// Finding inlined relations from the existing PSL definition,
/// matching the ones found in the database.
fn match_existing_inline_relations<'a>(
    sql_schema: &'a sql::SqlSchema,
    prisma_schema: &'a psl::ValidatedSchema,
    map: &mut IntrospectionMap<'_>,
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
    ctx: &DatamodelCalculatorContext<'_>,
    map: &mut IntrospectionMap<'_>,
) {
    map.existing_m2m_relations = sql_schema
        .table_walkers()
        .filter(|t| helpers::is_prisma_m_to_n_relation(*t, ctx.flavour.uses_pk_in_m2m_join_tables(ctx)))
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
