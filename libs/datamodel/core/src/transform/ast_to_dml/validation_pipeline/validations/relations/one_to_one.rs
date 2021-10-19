use super::*;
use crate::{
    diagnostics::{DatamodelError, Diagnostics},
    transform::ast_to_dml::db::walkers::InlineRelationWalker,
};

pub(crate) fn validate_strict(relation: InlineRelationWalker<'_, '_>, diagnostics: &mut Diagnostics) {
    let forward_relation_field = relation.forward_relation_field().expect(STATE_ERROR);
    let forward_relation_attributes = forward_relation_field.attributes();

    let back_relation_field = if let Some(back_relation_field) = relation.back_relation_field() {
        back_relation_field
    } else {
        let message = format!(
                "The relation field `{}` on Model `{}` is missing an opposite relation field on the model `{}`. Either run `prisma format` or add it manually.",
                &forward_relation_field.name(),
                &forward_relation_field.model().name(),
                &forward_relation_field.related_model().name(),
            );

        diagnostics.push_error(DatamodelError::new_field_validation_error(
            &message,
            forward_relation_field.model().name(),
            forward_relation_field.name(),
            forward_relation_field.ast_field().span,
        ));
        return;
    };

    let back_relation_attributes = back_relation_field.attributes();
    let mut errors = Vec::new();

    if is_empty_fields(forward_relation_attributes.fields.as_deref()) {
        let message = format!(
            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` do not provide the `fields` argument in the {} attribute. You have to provide it on one of the two fields.",
            &forward_relation_field.name(), &forward_relation_field.model().name(), &back_relation_field.name(), &back_relation_field.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT
        );

        errors.push(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward_relation_field.ast_field().span,
        ));

        if is_empty_fields(back_relation_attributes.fields.as_deref()) {
            // Do the same on the other field.

            let message = format!(
            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` do not provide the `fields` argument in the {} attribute. You have to provide it on one of the two fields.",
            &back_relation_field.name(), &back_relation_field.model().name(), &forward_relation_field.name(), &forward_relation_field.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT
        );

            errors.push(DatamodelError::new_attribute_validation_error(
                &message,
                RELATION_ATTRIBUTE_NAME,
                back_relation_field.ast_field().span,
            ));
        }
    }

    if is_empty_fields(forward_relation_attributes.references.as_deref())
        && is_empty_fields(back_relation_attributes.references.as_deref())
    {
        let message = format!(
            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` do not provide the `references` argument in the {} attribute. You have to provide it on one of the two fields.",
            &forward_relation_field.name(), &forward_relation_field.model().name(), &back_relation_field.name(), &back_relation_field.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT
        );

        errors.push(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward_relation_field.ast_field().span,
        ));

        // Same message on the other field.

        let message = format!(
            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` do not provide the `references` argument in the {} attribute. You have to provide it on one of the two fields.",
            &back_relation_field.name(), &back_relation_field.model().name(), &forward_relation_field.name(), &forward_relation_field.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT
        );

        errors.push(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back_relation_field.ast_field().span,
        ));
    }

    if !is_empty_fields(forward_relation_attributes.references.as_deref())
        && !is_empty_fields(back_relation_attributes.references.as_deref())
    {
        let message = format!(
                            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` both provide the `references` argument in the {} attribute. You have to provide it only on one of the two fields.",
                            &forward_relation_field.name(), &forward_relation_field.model().name(), &back_relation_field.name(), &back_relation_field.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT
                        );

        errors.push(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward_relation_field.ast_field().span,
        ));
        errors.push(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back_relation_field.ast_field().span,
        ));
    }

    if (forward_relation_field.relation_field.on_delete.is_some()
        || forward_relation_field.relation_field.on_update.is_some())
        && (back_relation_field.relation_field.on_delete.is_some()
            || back_relation_field.relation_field.on_update.is_some())
    {
        // We show the error on both fields
        let message = format!(
            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` both provide the `onDelete` or `onUpdate` argument in the {} attribute. You have to provide it only on one of the two fields.",
            &back_relation_field.name(), &back_relation_field.model().name(), &forward_relation_field.name(), &forward_relation_field.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT
        );

        errors.push(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back_relation_field.ast_field().span,
        ));

        let message = format!(
                            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` both provide the `onDelete` or `onUpdate` argument in the {} attribute. You have to provide it only on one of the two fields.",
                            &forward_relation_field.name(), &forward_relation_field.model().name(), &back_relation_field.name(), &back_relation_field.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT
                        );

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward_relation_field.ast_field().span,
        ));
    } else if back_relation_field.relation_field.on_delete.is_some()
        || back_relation_field.relation_field.on_update.is_some()
    {
        let message = &format!(
                            "The relation field `{}` on Model `{}` must not specify the `onDelete` or `onUpdate` argument in the {} attribute. You must only specify it on the opposite field `{}` on model `{}`.",
                            back_relation_field.name(), back_relation_field.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT, forward_relation_field.name(), forward_relation_field.model().name()
                        );

        errors.push(DatamodelError::new_attribute_validation_error(
            message,
            RELATION_ATTRIBUTE_NAME,
            back_relation_field.ast_field().span,
        ));
    }

    if !is_empty_fields(forward_relation_attributes.fields.as_deref())
        && !is_empty_fields(back_relation_attributes.fields.as_deref())
    {
        let message = format!(
            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` both provide the `fields` argument in the {} attribute. You have to provide it only on one of the two fields.",
            forward_relation_field.name(), forward_relation_field.model().name(), back_relation_field.name(), back_relation_field.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT
        );

        errors.push(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward_relation_field.ast_field().span,
        ));
        errors.push(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back_relation_field.ast_field().span,
        ));
    }

    if !errors.is_empty() {
        for error in errors {
            diagnostics.push_error(error);
        }
        return;
    }

    if !is_empty_fields(forward_relation_attributes.fields.as_deref())
        && !is_empty_fields(back_relation_attributes.references.as_deref())
    {
        let message = format!(
            "The relation field `{}` on Model `{}` provides the `fields` argument in the {} attribute. And the related field `{}` on Model `{}` provides the `references` argument. You must provide both arguments on the same side.",
            forward_relation_field.name(), forward_relation_field.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT, back_relation_field.name(), back_relation_field.model().name(),
        );

        errors.push(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward_relation_field.ast_field().span,
        ));
        errors.push(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back_relation_field.ast_field().span,
        ));
    }

    if !is_empty_fields(forward_relation_attributes.references.as_deref())
        && !is_empty_fields(back_relation_attributes.fields.as_deref())
    {
        let message = format!(
            "The relation field `{}` on Model `{}` provides the `references` argument in the {} attribute. And the related field `{}` on Model `{}` provides the `fields` argument. You must provide both arguments on the same side.",
            forward_relation_field.name(), forward_relation_field.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT, back_relation_field.name(), back_relation_field.model().name(),
        );

        errors.push(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward_relation_field.ast_field().span,
        ));
    }

    if !errors.is_empty() {
        for error in errors {
            diagnostics.push_error(error);
        }
        return;
    }

    if back_relation_field.ast_field().arity.is_required() {
        let message = format!(
            "The relation field `{}` on Model `{}` is required. This is no longer valid because it's not possible to enforce this constraint on the database level. Please change the field type from `{}` to `{}?` to fix this.",
            back_relation_field.name(), back_relation_field.model().name(), forward_relation_field.model().name(), forward_relation_field.model().name(),
        );

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back_relation_field.ast_field().span,
        ));
    }
}
