//! Rendering of model level index definitions.

use crate::{
    introspection_helpers::compare_options_none_last,
    pair::{IndexOps, ModelPair},
};
use datamodel_renderer::datamodel as renderer;
use sql_schema_describer as sql;

/// Renders `@@index`, `@@unique` and `@@fulltext` model level index
/// definitions.
pub(super) fn render<'a>(model: ModelPair<'a>, rendered: &mut renderer::Model<'a>) {
    // (Position in the existing model, index definition)
    let mut ordered_indexes: Vec<(Option<_>, renderer::IndexDefinition<'a>)> = Vec::new();

    for index in model.indexes() {
        let fields = index.fields().map(|field| {
            let mut definition = renderer::IndexFieldInput::new(field.name());

            if let Some(sort_order) = field.sort_order() {
                definition.sort_order(sort_order);
            }

            if let Some(length) = field.length() {
                definition.length(length);
            }

            if let Some(ops) = field.opclass() {
                let ops = match ops {
                    IndexOps::Managed(ops) => renderer::IndexOps::managed(ops),
                    IndexOps::Raw(ops) => renderer::IndexOps::raw(ops),
                };

                definition.ops(ops);
            }

            definition
        });

        let mut definition = match index.index_type() {
            sql::IndexType::Unique => renderer::IndexDefinition::unique(fields),
            sql::IndexType::Fulltext => renderer::IndexDefinition::fulltext(fields),
            sql::IndexType::Normal => renderer::IndexDefinition::index(fields),
            // we filter these out in the pair
            sql::IndexType::PrimaryKey => unreachable!(),
        };

        if let Some(name) = index.name() {
            definition.name(name);
        }

        if let Some(map) = index.mapped_name() {
            definition.map(map);
        }

        if let Some(clustered) = index.clustered() {
            definition.clustered(clustered);
        }

        if let Some(algo) = index.algorithm() {
            definition.index_type(algo);
        }

        ordered_indexes.push((index.previous_position(), definition));
    }

    ordered_indexes.sort_by(|(idx, _), (idx_b, _)| compare_options_none_last(*idx, *idx_b));

    for (_, definition) in ordered_indexes {
        rendered.push_index(definition);
    }
}
