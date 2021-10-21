use std::borrow::Cow;

use itertools::Itertools;

use crate::{
    diagnostics::{DatamodelError, Diagnostics},
    transform::ast_to_dml::db::walkers::ModelWalker,
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
