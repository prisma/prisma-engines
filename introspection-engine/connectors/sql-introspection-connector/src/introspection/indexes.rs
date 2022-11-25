use crate::{
    calculate_datamodel::InputContext,
    introspection_helpers::{compare_options_none_last, render_index},
};
use datamodel_renderer::datamodel as renderer;
use psl::{datamodel_connector::walker_ext_traits::IndexWalkerExt, parser_database::walkers};
use sql_schema_describer as sql;

pub(super) fn render_model_indexes<'a>(
    table: sql::TableWalker<'a>,
    existing_model: Option<walkers::ModelWalker<'a>>,
    model: &mut renderer::Model<'a>,
    input: InputContext<'a>,
) {
    // (Position in the existing model, index definition)
    let mut ordered_indexes: Vec<(Option<_>, renderer::IndexDefinition<'a>)> =
        Vec::with_capacity(table.indexes().len());

    for index in table.indexes() {
        let existing_index = existing_model.and_then(|model| {
            model
                .indexes()
                .find(|model_index| existing_index_matches(*model_index, index.name(), input))
        });

        if let Some(definition) = render_index(index, existing_index, input) {
            let attrid = existing_index.map(|idx| idx.attribute_id());
            ordered_indexes.push((attrid, definition));
        }
    }

    ordered_indexes.sort_by(|(idx, _), (idx_b, _)| compare_options_none_last(*idx, *idx_b));

    for (_, definition) in ordered_indexes {
        model.push_index(definition);
    }
}

fn existing_index_matches(
    existing_index: walkers::IndexWalker<'_>,
    sql_constraint_name: &str,
    input: InputContext<'_>,
) -> bool {
    // Upgrade logic. Prior to Prisma 3, PSL index attributes had a `name` argument but no `map`
    // argument. If we infer that an index in the database was produced using that logic, we
    // match up the existing index.
    if existing_index.mapped_name().is_none() && existing_index.name() == Some(sql_constraint_name) {
        return true;
    }

    // Compare the constraint name (implicit or mapped name) from the Prisma schema with the
    // constraint name from the database.
    existing_index.constraint_name(input.active_connector()) == sql_constraint_name
}
