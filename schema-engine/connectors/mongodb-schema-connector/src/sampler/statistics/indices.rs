use datamodel_renderer as renderer;
use mongodb_schema_describer::{IndexFieldProperty, IndexType, IndexWalker};
use psl::datamodel_connector::constraint_names::ConstraintNames;
use renderer::datamodel::{IndexDefinition, IndexFieldInput, Model};
use std::borrow::Cow;

pub(super) fn render<'a>(model: &mut Model<'a>, model_name: &str, indices: impl Iterator<Item = &'a IndexWalker<'a>>) {
    for index in indices {
        let fields = index.fields().map(|field| {
            let name = field
                .name()
                .split('.')
                .map(|part| {
                    super::sanitize_string(part)
                        .map(Cow::Owned)
                        .unwrap_or_else(|| Cow::Borrowed(part))
                })
                .collect::<Vec<_>>()
                .join(".");

            let mut rendered = IndexFieldInput::new(name);

            match field.property {
                IndexFieldProperty::Text => (),
                IndexFieldProperty::Ascending if index.r#type().is_fulltext() => {
                    rendered.sort_order("Asc");
                }
                IndexFieldProperty::Descending => {
                    rendered.sort_order("Desc");
                }
                IndexFieldProperty::Ascending => (),
            }

            rendered
        });

        let mut rendered = match index.r#type() {
            IndexType::Normal => IndexDefinition::index(fields),
            IndexType::Unique => IndexDefinition::unique(fields),
            IndexType::Fulltext => IndexDefinition::fulltext(fields),
        };

        let column_names = index.fields().flat_map(|f| f.name().split('.')).collect::<Vec<_>>();

        let default_name = match index.r#type() {
            IndexType::Unique => {
                ConstraintNames::unique_index_name(model_name, &column_names, psl::builtin_connectors::MONGODB)
            }
            _ => ConstraintNames::non_unique_index_name(model_name, &column_names, psl::builtin_connectors::MONGODB),
        };

        if index.name() != default_name {
            rendered.map(index.name());
        };

        model.push_index(rendered);
    }
}
