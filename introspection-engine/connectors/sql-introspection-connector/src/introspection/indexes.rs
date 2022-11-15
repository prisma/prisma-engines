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
    let mut ordered_indexes: Vec<(Option<usize>, dml::IndexDefinition)> = Vec::with_capacity(table.indexes().len());

    for index in table.indexes() {
        let existing_index = existing_model.and_then(|model| {
            model
                .indexes()
                .position(|model_index| model_index.constraint_name(ctx.active_connector()) == index.name())
        });

        if let Some(index_def) = calculate_index(index, ctx) {
            ordered_indexes.push((existing_index, index_def));
        }
    }

    ordered_indexes.sort_by(|(idx, _), (idx_b, _)| compare_options_none_last(*idx, *idx_b));

    model.indices = ordered_indexes.into_iter().map(|(_, idx)| idx).collect();
}
