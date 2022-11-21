use super::Context;
use crate::introspection_helpers::{calculate_index, compare_options_none_last};
use psl::{datamodel_connector::walker_ext_traits::IndexWalkerExt, dml, parser_database::walkers};
use sql_schema_describer as sql;

pub(super) fn calculate_model_indexes(
    table: sql::TableWalker<'_>,
    existing_model: Option<walkers::ModelWalker<'_>>,
    model: &mut dml::Model,
    ctx: &mut Context<'_>,
) {
    // (Position in the existing model, index definition)
    let mut ordered_indexes: Vec<(Option<_>, dml::IndexDefinition)> = Vec::with_capacity(table.indexes().len());

    for index in table.indexes() {
        let sql_constraint_name = index.name();
        let existing_index = existing_model.and_then(|model| {
            model
                .indexes()
                .find(|model_index| existing_index_matches(*model_index, sql_constraint_name, ctx))
        });
        let attrid = existing_index.map(|idx| idx.attribute_id());

        if let Some(index_def) = calculate_index(index, existing_index, ctx) {
            ordered_indexes.push((attrid, index_def));
        }
    }

    ordered_indexes.sort_by(|(idx, _), (idx_b, _)| compare_options_none_last(*idx, *idx_b));

    model.indices = ordered_indexes.into_iter().map(|(_, idx)| idx).collect();
}

fn existing_index_matches(
    existing_index: walkers::IndexWalker<'_>,
    sql_constraint_name: &str,
    ctx: &mut Context<'_>,
) -> bool {
    // Upgrade logic. Prior to Prisma 3, PSL index attributes had a `name` argument but no `map`
    // argument. If we infer that an index in the database was produced using that logic, we
    // match up the existing index.
    if existing_index.mapped_name().is_none() && existing_index.name() == Some(sql_constraint_name) {
        return true;
    }

    // Compare the constraint name (implicit or mapped name) from the Prisma schema with the
    // constraint name from the database.
    existing_index.constraint_name(ctx.active_connector()) == sql_constraint_name
}
