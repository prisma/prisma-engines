use crate::introspection::introspection_pair::IdPair;
use datamodel_renderer::datamodel as renderer;

/// Render a model/view level `@@id` definition.
pub(super) fn render(id: IdPair<'_>) -> renderer::IdDefinition<'_> {
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
    }

    if let Some(map) = id.mapped_name() {
        definition.map(map);
    }

    if let Some(clustered) = id.clustered() {
        definition.clustered(clustered);
    }

    definition
}
