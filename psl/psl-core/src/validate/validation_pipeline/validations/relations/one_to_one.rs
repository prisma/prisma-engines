use super::*;
use crate::{diagnostics::DatamodelError, validate::validation_pipeline::context::Context};
use parser_database::ast::WithSpan;

/// A relation should have the explicit and back-relation side defined.
pub(crate) fn both_sides_are_defined(relation: InlineRelationWalker<'_>, ctx: &mut Context<'_>) {
    if relation.back_relation_field().is_some() {
        return;
    }

    let field = relation.forward_relation_field().expect(STATE_ERROR);

    let container = if field.model().ast_model().is_view() {
        "view"
    } else {
        "model"
    };

    let message = format!(
        "The relation field `{}` on {container} `{}` is missing an opposite relation field on the model `{}`. Either run `prisma format` or add it manually.",
        field.name(),
        field.model().name(),
        field.related_model().name(),
    );

    ctx.push_error(DatamodelError::new_field_validation_error(
        &message,
        container,
        field.model().name(),
        field.name(),
        field.ast_field().span(),
    ));
}

/// The forward side must define `fields` and `references` in the `@relation` attribute.
pub(crate) fn fields_and_references_are_defined(relation: InlineRelationWalker<'_>, ctx: &mut Context<'_>) {
    let (forward, back) = match (relation.forward_relation_field(), relation.back_relation_field()) {
        (Some(forward), Some(back)) => (forward, back),
        _ => return,
    };

    if is_empty_fields(forward.referencing_fields()) && is_empty_fields(back.referencing_fields()) {
        let message = format!(
            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` do not provide the `fields` argument in the {} attribute. You have to provide it on one of the two fields.",
            forward.name(), forward.model().name(), back.name(), &back.model().name(), RELATION_ATTRIBUTE_NAME
        );

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward.ast_field().span(),
        ));

        // Do the same on the other field.

        let message = format!(
                "The relation fields `{}` on Model `{}` and `{}` on Model `{}` do not provide the `fields` argument in the {} attribute. You have to provide it on one of the two fields.",
                back.name(), back.model().name(), forward.name(), forward.model().name(), RELATION_ATTRIBUTE_NAME
            );

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back.ast_field().span(),
        ));
    }

    if is_empty_fields(forward.referenced_fields()) && is_empty_fields(back.referenced_fields()) {
        let message = format!(
            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` do not provide the `references` argument in the {} attribute. You have to provide it on one of the two fields.",
            forward.name(), forward.model().name(), back.name(), back.model().name(), RELATION_ATTRIBUTE_NAME
        );

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward.ast_field().span(),
        ));

        // Same message on the other field.

        let message = format!(
            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` do not provide the `references` argument in the {} attribute. You have to provide it on one of the two fields.",
            back.name(), back.model().name(), forward.name(), forward.model().name(), RELATION_ATTRIBUTE_NAME
        );

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back.ast_field().span(),
        ));
    }
}

/// `fields` and `references` should only be defined in the forward side of the relation.
pub(crate) fn fields_and_references_defined_on_one_side_only(
    relation: InlineRelationWalker<'_>,
    ctx: &mut Context<'_>,
) {
    let (forward, back) = match (relation.forward_relation_field(), relation.back_relation_field()) {
        (Some(forward), Some(back)) => (forward, back),
        _ => return,
    };

    if !is_empty_fields(forward.referenced_fields()) && !is_empty_fields(back.referenced_fields()) {
        let message = format!(
            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` both provide the `references` argument in the {} attribute. You have to provide it only on one of the two fields.",
            forward.name(), forward.model().name(), back.name(), back.model().name(), RELATION_ATTRIBUTE_NAME
        );

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward.ast_field().span(),
        ));

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back.ast_field().span(),
        ));
    }

    if !is_empty_fields(forward.referencing_fields()) && !is_empty_fields(back.referencing_fields()) {
        let message = format!(
            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` both provide the `fields` argument in the {} attribute. You have to provide it only on one of the two fields.",
            forward.name(), forward.model().name(), back.name(), back.model().name(), RELATION_ATTRIBUTE_NAME
        );

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward.ast_field().span(),
        ));

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back.ast_field().span(),
        ));
    }
}

/// Referential actions must be defined in the forward side.
pub(crate) fn referential_actions(relation: InlineRelationWalker<'_>, ctx: &mut Context<'_>) {
    let (forward, back) = match (relation.forward_relation_field(), relation.back_relation_field()) {
        (Some(forward), Some(back)) => (forward, back),
        _ => return,
    };

    if (forward.explicit_on_delete().is_some() || forward.explicit_on_update().is_some())
        && (back.explicit_on_delete().is_some() || back.explicit_on_update().is_some())
    {
        // We show the error on both fields
        let message = format!(
            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` both provide the `onDelete` or `onUpdate` argument in the {} attribute. You have to provide it only on one of the two fields.",
            back.name(), back.model().name(), forward.name(), forward.model().name(), RELATION_ATTRIBUTE_NAME
        );

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back.ast_field().span(),
        ));

        let message = format!(
            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` both provide the `onDelete` or `onUpdate` argument in the {} attribute. You have to provide it only on one of the two fields.",
            forward.name(), forward.model().name(), back.name(), back.model().name(), RELATION_ATTRIBUTE_NAME
        );

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward.ast_field().span(),
        ));
    } else if back.explicit_on_delete().is_some() || back.explicit_on_update().is_some() {
        let message = &format!(
            "The relation field `{}` on Model `{}` must not specify the `onDelete` or `onUpdate` argument in the {} attribute. You must only specify it on the opposite field `{}` on model `{}`.",
            back.name(), back.model().name(), RELATION_ATTRIBUTE_NAME, forward.name(), forward.model().name()
        );

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            message,
            RELATION_ATTRIBUTE_NAME,
            back.ast_field().span(),
        ));
    }
}

/// Validation of some crazy things, such as definining `fields` and `references` on different
/// sides in the relation.
pub(crate) fn fields_references_mixups(relation: InlineRelationWalker<'_>, ctx: &mut Context<'_>) {
    let (forward, back) = match (relation.forward_relation_field(), relation.back_relation_field()) {
        (Some(forward), Some(back)) if ctx.diagnostics.errors().is_empty() => (forward, back),
        _ => return,
    };

    if !is_empty_fields(forward.referencing_fields()) && !is_empty_fields(back.referenced_fields()) {
        let message = format!(
            "The relation field `{}` on Model `{}` provides the `fields` argument in the {} attribute. And the related field `{}` on Model `{}` provides the `references` argument. You must provide both arguments on the same side.",
            forward.name(), forward.model().name(), RELATION_ATTRIBUTE_NAME, back.name(), back.model().name(),
        );

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward.ast_field().span(),
        ));

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back.ast_field().span(),
        ));
    }

    if !is_empty_fields(forward.referenced_fields()) && !is_empty_fields(back.referencing_fields()) {
        let message = format!(
            "The relation field `{}` on Model `{}` provides the `references` argument in the {} attribute. And the related field `{}` on Model `{}` provides the `fields` argument. You must provide both arguments on the same side.",
            forward.name(), forward.model().name(), RELATION_ATTRIBUTE_NAME, back.name(), back.model().name(),
        );

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            forward.ast_field().span(),
        ));
    }
}

/// The back-relation side cannot be required.
pub(crate) fn back_relation_arity_is_optional(relation: InlineRelationWalker<'_>, ctx: &mut Context<'_>) {
    let (forward, back) = match (relation.forward_relation_field(), relation.back_relation_field()) {
        (Some(forward), Some(back)) if ctx.diagnostics.errors().is_empty() => (forward, back),
        _ => return,
    };

    if back.ast_field().arity.is_required() {
        let message = format!(
            "The relation field `{}` on Model `{}` is required. This is not valid because it's not possible to enforce this constraint on the database level. Please change the field type from `{}` to `{}?` to fix this.",
            back.name(), back.model().name(), forward.model().name(), forward.model().name(),
        );

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back.ast_field().span(),
        ));
    }
}

pub(crate) fn fields_and_references_on_wrong_side(relation: InlineRelationWalker<'_>, ctx: &mut Context<'_>) {
    let (forward, back) = match (relation.forward_relation_field(), relation.back_relation_field()) {
        (Some(forward), Some(back)) if ctx.diagnostics.errors().is_empty() => (forward, back),
        _ => return,
    };

    if forward.is_required() && (back.referencing_fields().is_some() || back.referenced_fields().is_some()) {
        let message = format!(
            "The relation field `{back_model}.{back_field}` defines the `fields` and/or `references` argument. You must set them on the required side of the relation (`{forward_model}.{forward_field}`) in order for the constraints to be enforced. Alternatively, you can change this field to be required and the opposite optional, or make both sides of the relation optional.",
            back_model = back.model().name(),
            back_field = back.name(),
            forward_model = forward.model().name(),
            forward_field = forward.name(),
        );

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            RELATION_ATTRIBUTE_NAME,
            back.ast_field().span(),
        ));
    }
}

/// A 1:1 relation is enforced with a unique constraint. The
/// referencing side must use a unique constraint to enforce the
/// relation.
pub(crate) fn fields_must_be_a_unique_constraint(relation: InlineRelationWalker<'_>, ctx: &mut Context<'_>) {
    let forward = match relation.forward_relation_field() {
        Some(field) => field,
        None => return,
    };

    let model = relation.referencing_model();

    let is_unique = model.unique_criterias().any(|c| {
        let fields = if let Some(fields) = relation.referencing_fields() {
            fields
        } else {
            return true;
        };

        c.contains_exactly_fields(fields)
    });

    if is_unique {
        return;
    }

    let fields = if let Some(fields) = relation.referencing_fields() {
        fields.map(|f| f.name()).collect::<Vec<_>>()
    } else {
        return;
    };

    let message = if fields.len() == 1 {
        format!("A one-to-one relation must use unique fields on the defining side. Either add an `@unique` attribute to the field `{}`, or change the relation to one-to-many.", fields.join(", "))
    } else {
        format!("A one-to-one relation must use unique fields on the defining side. Either add an `@@unique([{}])` attribute to the model, or change the relation to one-to-many.", fields.join(", "))
    };

    ctx.push_error(DatamodelError::new_attribute_validation_error(
        &message,
        RELATION_ATTRIBUTE_NAME,
        forward.ast_field().span(),
    ));
}
