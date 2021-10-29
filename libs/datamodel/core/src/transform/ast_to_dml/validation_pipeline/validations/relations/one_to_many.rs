use super::*;
use crate::{
    diagnostics::{DatamodelError, Diagnostics},
    transform::ast_to_dml::db::walkers::{InlineRelationWalker, RelationFieldWalker},
};

/// A relation must be defined from both sides, one defining the fields, references and possible
/// referential actions, the other side just as a list.
pub(crate) fn both_sides_are_defined(relation: InlineRelationWalker<'_, '_>, diagnostics: &mut Diagnostics) {
    let mut error_fn = |relation_field: RelationFieldWalker<'_, '_>| {
        let message = format!(
            "The relation field `{}` on Model `{}` is missing an opposite relation field on the model `{}`. Either run `prisma format` or add it manually.",
            &relation_field.name(),
            &relation_field.model().name(),
            &relation_field.related_model().name(),
            );

        diagnostics.push_error(DatamodelError::new_field_validation_error(
            &message,
            relation_field.model().name(),
            relation_field.name(),
            relation_field.ast_field().span,
        ));
    };

    match (relation.forward_relation_field(), relation.back_relation_field()) {
        (Some(forward_relation_field), None) => error_fn(forward_relation_field),
        (None, Some(back_relation_field)) => error_fn(back_relation_field),
        _ => (),
    }
}

/// The singular side must define `fields` and `references` attributes.
pub(crate) fn fields_and_references_are_defined(relation: InlineRelationWalker<'_, '_>, diagnostics: &mut Diagnostics) {
    let (forward, back) = match (relation.forward_relation_field(), relation.back_relation_field()) {
        (Some(forward), Some(back)) => (forward, back),
        _ => return,
    };

    // fields argument should not be empty
    if is_empty_fields(forward.attributes().fields.as_deref()) {
        let message = format!(
            "The relation field `{}` on Model `{}` must specify the `fields` argument in the {} attribute. {}",
            forward.name(),
            forward.model().name(),
            RELATION_ATTRIBUTE_NAME_WITH_AT,
            PRISMA_FORMAT_HINT
        );

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward.ast_field().span,
        ));
    }

    // references argument should not be empty
    if is_empty_fields(forward.attributes().references.as_deref()) {
        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &format!(
                "The relation field `{}` on Model `{}` must specify the `references` argument in the {} attribute.",
                forward.name(),
                forward.model().name(),
                RELATION_ATTRIBUTE_NAME_WITH_AT
            ),
            RELATION_ATTRIBUTE_NAME,
            forward.ast_field().span,
        ));
    }

    if !is_empty_fields(back.attributes().fields.as_deref())
        || !is_empty_fields(back.attributes().references.as_deref())
    {
        let message = format!(
            "The relation field `{}` on Model `{}` must not specify the `fields` or `references` argument in the {} attribute. You must only specify it on the opposite field `{}` on model `{}`.",
            back.name(),
            back.model().name(),
            RELATION_ATTRIBUTE_NAME_WITH_AT,
            forward.name(),
            forward.model().name(),
        );

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back.ast_field().span,
        ));
    }
}

/// The referential actions, if defined, must be on the singular side only.
pub(crate) fn referential_actions(relation: InlineRelationWalker<'_, '_>, diagnostics: &mut Diagnostics) {
    let (forward, back) = match (relation.forward_relation_field(), relation.back_relation_field()) {
        (Some(forward), Some(back)) => (forward, back),
        _ => return,
    };

    if back.attributes().on_delete.is_some() || back.attributes().on_update.is_some() {
        let message = &format!(
            "The relation field `{}` on Model `{}` must not specify the `onDelete` or `onUpdate` argument in the {} attribute. You must only specify it on the opposite field `{}` on model `{}`, or in case of a many to many relation, in an explicit join table.",
            back.name(), back.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT, forward.name(), forward.model().name(),
        );

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            message,
            RELATION_ATTRIBUTE_NAME,
            back.ast_field().span,
        ));
    }
}
