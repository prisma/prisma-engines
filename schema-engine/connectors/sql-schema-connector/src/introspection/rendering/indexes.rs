//! Rendering of model level index definitions.

use crate::introspection::introspection_pair::{IndexOps, IndexPair};
use datamodel_renderer::datamodel as renderer;
use sql_schema_describer as sql;

pub(super) fn render(index: IndexPair<'_>) -> renderer::IndexDefinition<'_> {
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

    if let Some(predicate) = index.predicate() {
        definition.where_clause(predicate);
    }

    definition
}
