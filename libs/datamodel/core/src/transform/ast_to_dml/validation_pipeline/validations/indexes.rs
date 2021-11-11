use datamodel_connector::ConnectorCapability;

use crate::ast::Span;
use crate::{
    common::preview_features::PreviewFeature,
    diagnostics::{DatamodelError, Diagnostics},
    transform::ast_to_dml::db::{walkers::IndexWalker, ConstraintName, ParserDatabase},
};

/// Different databases validate index and unique constraint names in a certain namespace.
/// Validates index and unique constraint names against the database requirements.
pub(crate) fn has_a_unique_constraint_name(
    db: &ParserDatabase<'_>,
    index: IndexWalker<'_, '_>,
    diagnostics: &mut Diagnostics,
) {
    let name = index.final_database_name();
    let model = index.model();

    for violation in db.scope_violations(model.model_id(), ConstraintName::Index(name.as_ref())) {
        let message = format!(
            "The given constraint name `{}` has to be unique in the following namespace: {}. Please provide a different name using the `map` argument.",
            name,
            violation.description(model.name()),
        );

        let span = index
            .ast_attribute()
            .map(|a| {
                let from_arg = a.span_for_argument("map").or_else(|| a.span_for_argument("name"));
                from_arg.unwrap_or(a.span)
            })
            .unwrap_or_else(|| model.ast_model().span);

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &message,
            index.attribute_name(),
            span,
        ));
    }
}

/// sort and length are not yet allowed
pub(crate) fn uses_length_or_sort_without_preview_flag(
    db: &ParserDatabase<'_>,
    index: IndexWalker<'_, '_>,
    diagnostics: &mut Diagnostics,
) {
    if db.preview_features.contains(PreviewFeature::ExtendedIndexes) {
        return;
    }

    if index
        .scalar_field_attributes()
        .any(|f| f.sort_order().is_some() || f.length().is_some())
    {
        let message = "The sort and length arguments are not yet available.";

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            message,
            index.attribute_name(),
            index.ast_attribute().map(|i| i.span).unwrap_or_else(Span::empty),
        ));
    }
}

/// The database must support the index length prefix for it to be allowed in the data model.
pub(crate) fn field_length_prefix_supported(
    db: &ParserDatabase<'_>,
    index: IndexWalker<'_, '_>,
    diagnostics: &mut Diagnostics,
) {
    if db
        .active_connector()
        .has_capability(ConnectorCapability::IndexColumnLengthPrefixing)
    {
        return;
    }

    if index.scalar_field_attributes().any(|f| f.length().is_some()) {
        let message = "The length argument is not supported in an index definition with the current connector";
        let span = index.ast_attribute().map(|i| i.span).unwrap_or_else(Span::empty);

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            message,
            index.attribute_name(),
            span,
        ));
    }
}

//TODO(extended indices) add db specific validations to sort and length
