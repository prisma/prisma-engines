use super::*;
use crate::{
    diagnostics::{DatamodelError, Diagnostics},
    transform::ast_to_dml::db::walkers::InlineRelationWalker,
};

/// A relation should have the explicit and back-relation side defined.
pub(crate) fn both_sides_are_defined(relation: InlineRelationWalker<'_, '_>, diagnostics: &mut Diagnostics) {
    if relation.back_relation_field().is_some() {
        return;
    }

    let field = relation.forward_relation_field().expect(STATE_ERROR);

    let message = format!(
        "The relation field `{}` on Model `{}` is missing an opposite relation field on the model `{}`. Either run `prisma format` or add it manually.",
        field.name(),
        field.model().name(),
        field.related_model().name(),
    );

    diagnostics.push_error(DatamodelError::new_field_validation_error(
        &message,
        field.model().name(),
        field.name(),
        field.ast_field().span,
    ));
}

/// The forward side must define `fields` and `references` in the `@relation` attribute.
pub(crate) fn fields_and_references_are_defined(relation: InlineRelationWalker<'_, '_>, diagnostics: &mut Diagnostics) {
    let (forward, back) = match (relation.forward_relation_field(), relation.back_relation_field()) {
        (Some(forward), Some(back)) => (forward, back),
        _ => return,
    };

    if is_empty_fields(forward.attributes().fields.as_deref()) && is_empty_fields(back.attributes().fields.as_deref()) {
        let message = format!(
            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` do not provide the `fields` argument in the {} attribute. You have to provide it on one of the two fields.",
            forward.name(), forward.model().name(), back.name(), &back.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT
        );

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward.ast_field().span,
        ));

        // Do the same on the other field.

        let message = format!(
                "The relation fields `{}` on Model `{}` and `{}` on Model `{}` do not provide the `fields` argument in the {} attribute. You have to provide it on one of the two fields.",
                back.name(), back.model().name(), forward.name(), forward.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT
            );

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back.ast_field().span,
        ));
    }

    if is_empty_fields(forward.attributes().references.as_deref())
        && is_empty_fields(back.attributes().references.as_deref())
    {
        let message = format!(
            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` do not provide the `references` argument in the {} attribute. You have to provide it on one of the two fields.",
            forward.name(), forward.model().name(), back.name(), back.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT
        );

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward.ast_field().span,
        ));

        // Same message on the other field.

        let message = format!(
            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` do not provide the `references` argument in the {} attribute. You have to provide it on one of the two fields.",
            back.name(), back.model().name(), forward.name(), forward.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT
        );

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back.ast_field().span,
        ));
    }
}

/// `fields` and `references` should only be defined in the forward side of the relation.
pub(crate) fn fields_and_references_defined_on_one_side_only(
    relation: InlineRelationWalker<'_, '_>,
    diagnostics: &mut Diagnostics,
) {
    let (forward, back) = match (relation.forward_relation_field(), relation.back_relation_field()) {
        (Some(forward), Some(back)) => (forward, back),
        _ => return,
    };

    if !is_empty_fields(forward.attributes().references.as_deref())
        && !is_empty_fields(back.attributes().references.as_deref())
    {
        let message = format!(
            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` both provide the `references` argument in the {} attribute. You have to provide it only on one of the two fields.",
            forward.name(), forward.model().name(), back.name(), back.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT
        );

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward.ast_field().span,
        ));

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back.ast_field().span,
        ));
    }

    if !is_empty_fields(forward.attributes().fields.as_deref()) && !is_empty_fields(back.attributes().fields.as_deref())
    {
        let message = format!(
            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` both provide the `fields` argument in the {} attribute. You have to provide it only on one of the two fields.",
            forward.name(), forward.model().name(), back.name(), back.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT
        );

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward.ast_field().span,
        ));

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back.ast_field().span,
        ));
    }
}

/// Referential actions must be defined in the forward side.
pub(crate) fn referential_actions(relation: InlineRelationWalker<'_, '_>, diagnostics: &mut Diagnostics) {
    let (forward, back) = match (relation.forward_relation_field(), relation.back_relation_field()) {
        (Some(forward), Some(back)) => (forward, back),
        _ => return,
    };

    if (forward.attributes().on_delete.is_some() || forward.attributes().on_update.is_some())
        && (back.attributes().on_delete.is_some() || back.attributes().on_update.is_some())
    {
        // We show the error on both fields
        let message = format!(
            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` both provide the `onDelete` or `onUpdate` argument in the {} attribute. You have to provide it only on one of the two fields.",
            back.name(), back.model().name(), forward.name(), forward.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT
        );

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back.ast_field().span,
        ));

        let message = format!(
            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` both provide the `onDelete` or `onUpdate` argument in the {} attribute. You have to provide it only on one of the two fields.",
            forward.name(), forward.model().name(), back.name(), back.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT
        );

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward.ast_field().span,
        ));
    } else if back.attributes().on_delete.is_some() || back.attributes().on_update.is_some() {
        let message = &format!(
            "The relation field `{}` on Model `{}` must not specify the `onDelete` or `onUpdate` argument in the {} attribute. You must only specify it on the opposite field `{}` on model `{}`.",
            back.name(), back.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT, forward.name(), forward.model().name()
        );

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            message,
            RELATION_ATTRIBUTE_NAME,
            back.ast_field().span,
        ));
    }
}

/// Validation of some crazy things, such as definining `fields` and `references` on different
/// sides in the relation.
pub(crate) fn fields_references_mixups(relation: InlineRelationWalker<'_, '_>, diagnostics: &mut Diagnostics) {
    let (forward, back) = match (relation.forward_relation_field(), relation.back_relation_field()) {
        (Some(forward), Some(back)) if diagnostics.errors().is_empty() => (forward, back),
        _ => return,
    };

    if !is_empty_fields(forward.attributes().fields.as_deref())
        && !is_empty_fields(back.attributes().references.as_deref())
    {
        let message = format!(
            "The relation field `{}` on Model `{}` provides the `fields` argument in the {} attribute. And the related field `{}` on Model `{}` provides the `references` argument. You must provide both arguments on the same side.",
            forward.name(), forward.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT, back.name(), back.model().name(),
        );

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward.ast_field().span,
        ));

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back.ast_field().span,
        ));
    }

    if !is_empty_fields(forward.attributes().references.as_deref())
        && !is_empty_fields(back.attributes().fields.as_deref())
    {
        let message = format!(
            "The relation field `{}` on Model `{}` provides the `references` argument in the {} attribute. And the related field `{}` on Model `{}` provides the `fields` argument. You must provide both arguments on the same side.",
            forward.name(), forward.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT, back.name(), back.model().name(),
        );

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward.ast_field().span,
        ));
    }
}

/// The back-relation side cannot be required.
pub(crate) fn back_relation_arity_is_optional(relation: InlineRelationWalker<'_, '_>, diagnostics: &mut Diagnostics) {
    let (forward, back) = match (relation.forward_relation_field(), relation.back_relation_field()) {
        (Some(forward), Some(back)) if diagnostics.errors().is_empty() => (forward, back),
        _ => return,
    };

    if back.ast_field().arity.is_required() {
        let message = format!(
            "The relation field `{}` on Model `{}` is required. This is no longer valid because it's not possible to enforce this constraint on the database level. Please change the field type from `{}` to `{}?` to fix this.",
            back.name(), back.model().name(), forward.model().name(), forward.model().name(),
        );

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back.ast_field().span,
        ));
    }
}
