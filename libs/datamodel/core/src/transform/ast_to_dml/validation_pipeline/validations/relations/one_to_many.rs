use super::*;
use crate::{
    diagnostics::{DatamodelError, Diagnostics},
    transform::ast_to_dml::db::walkers::{InlineRelationWalker, RelationFieldWalker},
};

pub(crate) fn validate_strict(relation: InlineRelationWalker<'_, '_>, diagnostics: &mut Diagnostics) {
    match (relation.forward_relation_field(), relation.back_relation_field()) {
        (Some(forward_relation_field), None) => error_on_one_sided_relation(forward_relation_field, diagnostics),
        (None, Some(back_relation_field)) => {
            error_on_one_sided_relation(back_relation_field, diagnostics);
        }
        (Some(forward_relation_field), Some(back_relation_field)) => {
            forward_relation_field_validations(forward_relation_field, diagnostics);

            let back_relation_attributes = back_relation_field.attributes();

            if !is_empty_fields(back_relation_attributes.fields.as_deref())
                || !is_empty_fields(back_relation_attributes.references.as_deref())
            {
                let message = format!(
                        "The relation field `{}` on Model `{}` must not specify the `fields` or `references` argument in the {} attribute. You must only specify it on the opposite field `{}` on model `{}`.",
                        back_relation_field.name(),
                        back_relation_field.model().name(),
                        RELATION_ATTRIBUTE_NAME_WITH_AT,
                        forward_relation_field.name(),
                        forward_relation_field.model().name(),
                    );

                diagnostics.push_error(DatamodelError::new_attribute_validation_error(
                    &message,
                    RELATION_ATTRIBUTE_NAME,
                    back_relation_field.ast_field().span,
                ));
            }

            if back_relation_attributes.on_delete.is_some() || back_relation_attributes.on_update.is_some() {
                let message = &format!(
                        "The relation field `{}` on Model `{}` must not specify the `onDelete` or `onUpdate` argument in the {} attribute. You must only specify it on the opposite field `{}` on model `{}`, or in case of a many to many relation, in an explicit join table.",
                        back_relation_field.name(), &back_relation_field.model().name(), RELATION_ATTRIBUTE_NAME_WITH_AT, forward_relation_field.name(), forward_relation_field.model().name(),
                    );

                diagnostics.push_error(DatamodelError::new_attribute_validation_error(
                    message,
                    RELATION_ATTRIBUTE_NAME,
                    back_relation_field.ast_field().span,
                ));
            }
        }
        (None, None) => unreachable!(),
    }
}

fn forward_relation_field_validations(
    forward_relation_field: RelationFieldWalker<'_, '_>,
    diagnostics: &mut Diagnostics,
) {
    let forward_relation_attributes = forward_relation_field.attributes();

    // fields argument should not be empty
    if is_empty_fields(forward_relation_attributes.fields.as_deref()) {
        let message = format!(
            "The relation field `{}` on Model `{}` must specify the `fields` argument in the {} attribute. {}",
            forward_relation_field.name(),
            &forward_relation_field.model().name(),
            RELATION_ATTRIBUTE_NAME_WITH_AT,
            PRISMA_FORMAT_HINT
        );

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward_relation_field.ast_field().span,
        ));
    }

    // references argument should not be empty
    if is_empty_fields(forward_relation_attributes.references.as_deref()) {
        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &format!(
                "The relation field `{}` on Model `{}` must specify the `references` argument in the {} attribute.",
                forward_relation_field.name(),
                &forward_relation_field.model().name(),
                RELATION_ATTRIBUTE_NAME_WITH_AT
            ),
            RELATION_ATTRIBUTE_NAME,
            forward_relation_field.ast_field().span,
        ));
    }
}

fn error_on_one_sided_relation(relation_field: RelationFieldWalker<'_, '_>, diagnostics: &mut Diagnostics) {
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
}
