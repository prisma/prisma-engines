use std::borrow::Cow;

use itertools::Itertools;

use crate::{
    common::preview_features::PreviewFeature,
    diagnostics::{DatamodelError, Diagnostics},
    transform::ast_to_dml::db::{walkers::ModelWalker, ConstraintName, ParserDatabase},
};

/// A model must have either a primary key, or a unique criteria
/// with no optional, commented-out or unsupported fields.
pub(super) fn has_a_strict_unique_criteria(model: ModelWalker<'_, '_>, diagnostics: &mut Diagnostics) {
    if model.is_ignored() {
        return;
    }

    let strict_criteria = model
        .unique_criterias()
        .find(|c| c.is_strict_criteria() && !c.has_unsupported_fields());

    if strict_criteria.is_some() {
        return;
    }

    let mut loose_criterias = model
        .unique_criterias()
        .map(|c| {
            let mut field_names = c.fields().map(|c| c.name());
            format!("- {}", field_names.join(", "))
        })
        .peekable();

    let msg =
        "Each model must have at least one unique criteria that has only required fields. Either mark a single field with `@id`, `@unique` or add a multi field criterion with `@@id([])` or `@@unique([])` to the model.";

    let msg = if loose_criterias.peek().is_some() {
        let suffix = format!(
            "The following unique criterias were not considered as they contain fields that are not required:\n{}",
            loose_criterias.join("\n"),
        );

        Cow::from(format!("{} {}", msg, suffix))
    } else {
        Cow::from(msg)
    };

    diagnostics.push_error(DatamodelError::new_model_validation_error(
        msg.as_ref(),
        model.name(),
        model.ast_model().span,
    ))
}

/// A primary key name can be unique in different namespaces, depending on a database. Validates
/// model's primary key against the database requirements.
pub(crate) fn has_a_unique_primary_key_name(
    db: &ParserDatabase<'_>,
    model: ModelWalker<'_, '_>,
    diagnostics: &mut Diagnostics,
) {
    let (pk, name) = match model
        .primary_key()
        .and_then(|pk| pk.final_database_name().map(|name| (pk, name)))
    {
        Some((pk, name)) => (pk, name),
        None => return,
    };

    for violation in db.scope_violations(model.model_id(), ConstraintName::PrimaryKey(name.as_ref())) {
        let message = format!(
            "The given constraint name `{}` has to be unique in the following namespace: {}. Please provide a different name using the `map` argument.",
            name,
            violation.description(model.name())
        );

        let span = pk
            .ast_attribute()
            .span_for_argument("map")
            .unwrap_or_else(|| pk.ast_attribute().span);

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(&message, "id", span));
    }
}

/// uses sort or length on id without preview flag
pub(crate) fn uses_sort_or_length_on_primary_without_preview_flag(
    db: &ParserDatabase<'_>,
    model: ModelWalker<'_, '_>,
    diagnostics: &mut Diagnostics,
) {
    if let Some(pk) = model.primary_key() {
        if !db.preview_features.contains(PreviewFeature::ExtendedIndexes)
            && pk
                .attribute
                .fields
                .iter()
                .any(|f| f.sort_order.is_some() || f.length.is_some())
        {
            let message = "The sort and length args are not yet available";

            let span = pk.ast_attribute().span;

            diagnostics.push_error(DatamodelError::new_attribute_validation_error(message, "id", span));
        }
    }
}
