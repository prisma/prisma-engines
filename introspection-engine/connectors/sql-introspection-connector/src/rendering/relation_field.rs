//! Rendering of relation fields.

use crate::{pair::RelationFieldPair, warnings::Warnings};
use datamodel_renderer::datamodel as renderer;

/// Render a relation field to be added in a model.
pub(super) fn render<'a>(field: RelationFieldPair<'a>, warnings: &mut Warnings) -> renderer::ModelField<'a> {
    let mut rendered = renderer::ModelField::new(field.field_name(), field.prisma_type());

    if field.is_optional() {
        rendered.optional();
    } else if field.is_array() {
        rendered.array();
    }

    if field.ignore() {
        rendered.ignore();
    }

    if field.renders_attribute() {
        let mut relation = renderer::Relation::new();

        if let Some(name) = field.relation_name() {
            relation.name(name);
        }

        if let Some(fields) = field.fields() {
            relation.fields(fields);
        }

        if let Some(references) = field.references() {
            relation.references(references);
        }

        if let Some(action) = field.on_delete() {
            relation.on_delete(action);
        }

        if let Some(action) = field.on_update() {
            relation.on_update(action);
        }

        if let Some(map) = field.constraint_name() {
            relation.map(map);
        }

        rendered.relation(relation);
    }

    if field.reintrospected_relation() {
        warnings.reintrospected_relations.push(crate::warnings::Model {
            model: field.prisma_type().into_owned(),
        });
    }

    rendered
}
