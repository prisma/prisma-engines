use crate::{
    pair::IdPair,
    warnings::{self, Warnings},
};
use datamodel_renderer::datamodel as renderer;

/// Render a model/view level `@@id` definition.
pub(super) fn render<'a>(id: IdPair<'a>, warnings: &mut Warnings) -> renderer::IdDefinition<'a> {
    let fields = id.fields().map(|field| {
        let mut rendered = renderer::IndexFieldInput::new(field.name());

        if let Some(sort_order) = field.sort_order() {
            rendered.sort_order(sort_order);
        }

        if let Some(length) = field.length() {
            rendered.length(length);
        }

        rendered
    });

    let mut definition = renderer::IdDefinition::new(fields);

    if let Some(name) = id.name() {
        definition.name(name);

        match id.model() {
            Some(model) if model.ast_model().is_view() => {
                warnings.reintrospected_id_names_in_view.push(warnings::View {
                    view: model.name().to_string(),
                });
            }
            Some(model) => {
                warnings.reintrospected_id_names_in_model.push(warnings::Model {
                    model: model.name().to_string(),
                });
            }
            None => (),
        }
    }

    if let Some(map) = id.mapped_name() {
        definition.map(map);
    }

    if let Some(clustered) = id.clustered() {
        definition.clustered(clustered);
    }

    definition
}
