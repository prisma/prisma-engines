use super::*;
use crate::{diagnostics::DatamodelError, validate::validation_pipeline::context::Context};
use parser_database::{
    ast::WithSpan,
    walkers::{InlineRelationWalker, RelationFieldWalker},
};

/// A relation must be defined from both sides, one defining the fields, references and possible
/// referential actions, the other side just as a list.
pub(crate) fn both_sides_are_defined(relation: InlineRelationWalker<'_>, ctx: &mut Context<'_>) {
    let mut error_fn = |relation_field: RelationFieldWalker<'_>| {
        let container = if relation_field.model().ast_model().is_view() {
            "view"
        } else {
            "model"
        };

        let message = format!(
            "The relation field `{}` on {container} `{}` is missing an opposite relation field on the model `{}`. Either run `prisma format` or add it manually.",
            &relation_field.name(),
            &relation_field.model().name(),
            &relation_field.related_model().name(),
        );

        ctx.push_error(DatamodelError::new_field_validation_error(
            &message,
            container,
            relation_field.model().name(),
            relation_field.name(),
            relation_field.ast_field().span(),
        ));
    };

    match (relation.forward_relation_field(), relation.back_relation_field()) {
        (Some(forward_relation_field), None) => error_fn(forward_relation_field),
        (None, Some(back_relation_field)) => error_fn(back_relation_field),
        _ => (),
    }
}

/// The singular side must define `fields` and `references` attributes.
pub(crate) fn fields_and_references_are_defined(relation: InlineRelationWalker<'_>, ctx: &mut Context<'_>) {
    let (forward, back) = match (relation.forward_relation_field(), relation.back_relation_field()) {
        (Some(forward), Some(back)) => (forward, back),
        _ => return,
    };

    // fields argument should not be empty
    if is_empty_fields(forward.referencing_fields()) {
        let message = format!(
            "The relation field `{}` on Model `{}` must specify the `fields` argument in the {} attribute. {}",
            forward.name(),
            forward.model().name(),
            RELATION_ATTRIBUTE_NAME,
            PRISMA_FORMAT_HINT
        );

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward.ast_field().span(),
        ));
    }

    // references argument should not be empty
    if is_empty_fields(forward.referenced_fields()) {
        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &format!(
                "The relation field `{}` on Model `{}` must specify the `references` argument in the {} attribute.",
                forward.name(),
                forward.model().name(),
                RELATION_ATTRIBUTE_NAME
            ),
            RELATION_ATTRIBUTE_NAME,
            forward.ast_field().span(),
        ));
    }

    if !is_empty_fields(back.referencing_fields()) || !is_empty_fields(back.referenced_fields()) {
        let message = format!(
            "The relation field `{}` on Model `{}` must not specify the `fields` or `references` argument in the {} attribute. You must only specify it on the opposite field `{}` on model `{}`.",
            back.name(),
            back.model().name(),
            RELATION_ATTRIBUTE_NAME,
            forward.name(),
            forward.model().name(),
        );

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back.ast_field().span(),
        ));
    }
}

/// The referential actions, if defined, must be on the singular side only.
pub(crate) fn referential_actions(relation: InlineRelationWalker<'_>, ctx: &mut Context<'_>) {
    let (forward, back) = match (relation.forward_relation_field(), relation.back_relation_field()) {
        (Some(forward), Some(back)) => (forward, back),
        _ => return,
    };

    if back.explicit_on_delete().is_some() || back.explicit_on_update().is_some() {
        let message = &format!(
            "The relation field `{}` on Model `{}` must not specify the `onDelete` or `onUpdate` argument in the {} attribute. You must only specify it on the opposite field `{}` on model `{}`, or in case of a many to many relation, in an explicit join table.",
            back.name(), back.model().name(), RELATION_ATTRIBUTE_NAME, forward.name(), forward.model().name(),
        );

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            message,
            RELATION_ATTRIBUTE_NAME,
            back.ast_field().span(),
        ));
    }
}
