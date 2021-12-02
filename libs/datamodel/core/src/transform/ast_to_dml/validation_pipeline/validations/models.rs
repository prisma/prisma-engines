use std::borrow::Cow;

use datamodel_connector::{Connector, ConnectorCapability};
use itertools::Itertools;

use crate::{
    common::preview_features::PreviewFeature,
    diagnostics::{DatamodelError, Diagnostics},
    transform::ast_to_dml::db::{walkers::ModelWalker, ParserDatabase},
};

use super::database_name::validate_db_name;

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
pub(super) fn has_a_unique_primary_key_name(
    model: ModelWalker<'_, '_>,
    names: &super::Names<'_>,
    connector: &dyn Connector,
    diagnostics: &mut Diagnostics,
) {
    let (pk, name) = match model
        .primary_key()
        .and_then(|pk| pk.final_database_name(connector).map(|name| (pk, name)))
    {
        Some((pk, name)) => (pk, name),
        None => return,
    };

    validate_db_name(
        model.name(),
        pk.ast_attribute(),
        Some(&name),
        connector,
        diagnostics,
        !pk.is_defined_on_field(),
    );

    for violation in names.constraint_namespace.scope_violations(
        model.model_id(),
        super::constraint_namespace::ConstraintName::PrimaryKey(name.as_ref()),
    ) {
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
    if db.preview_features.contains(PreviewFeature::ExtendedIndexes) {
        return;
    }

    if let Some(pk) = model.primary_key() {
        if pk
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

/// The database must support the primary key length prefix for it to be allowed in the data model.
pub(crate) fn primary_key_length_prefix_supported(
    model: ModelWalker<'_, '_>,
    connector: &dyn Connector,
    diagnostics: &mut Diagnostics,
) {
    if connector.has_capability(ConnectorCapability::IndexColumnLengthPrefixing) {
        return;
    }

    if let Some(pk) = model.primary_key() {
        if pk.scalar_field_attributes().any(|f| f.length().is_some()) {
            let message = "The length argument is not supported in the primary key with the current connector";
            let span = pk.ast_attribute().span;

            diagnostics.push_error(DatamodelError::new_attribute_validation_error(message, "id", span));
        }
    }
}

/// Not every database is allowing sort definition in the primary key.
pub(crate) fn primary_key_sort_order_supported(
    model: ModelWalker<'_, '_>,
    connector: &dyn Connector,
    diagnostics: &mut Diagnostics,
) {
    if connector.has_capability(ConnectorCapability::PrimaryKeySortOrderDefinition) {
        return;
    }

    if let Some(pk) = model.primary_key() {
        if pk.scalar_field_attributes().any(|f| f.sort_order().is_some()) {
            let message = "The sort argument is not supported in the primary key with the current connector";
            let span = pk.ast_attribute().span;

            diagnostics.push_error(DatamodelError::new_attribute_validation_error(message, "id", span));
        }
    }
}

pub(crate) fn only_one_fulltext_attribute_allowed(
    db: &ParserDatabase<'_>,
    model: ModelWalker<'_, '_>,
    connector: &dyn Connector,
    diagnostics: &mut Diagnostics,
) {
    if !db.preview_features.contains(PreviewFeature::FullTextIndex) {
        return;
    }

    if !connector.has_capability(ConnectorCapability::FullTextIndex) {
        return;
    }

    if connector.has_capability(ConnectorCapability::MultipleFullTextAttributesPerModel) {
        return;
    }

    let spans = model
        .indexes()
        .filter(|i| i.is_fulltext())
        .map(|i| i.ast_attribute().map(|i| i.span).unwrap_or(model.ast_model().span))
        .collect::<Vec<_>>();

    if spans.len() > 1 {
        for span in spans {
            let message = "The current connector only allows one fulltext attribute per model";

            diagnostics.push_error(DatamodelError::new_attribute_validation_error(
                message, "fulltext", span,
            ));
        }
    }
}

/// Does the connector support named and compound primary keys at all?
pub(crate) fn primary_key_connector_specific(
    model: ModelWalker<'_, '_>,
    connector: &dyn Connector,
    diagnostics: &mut Diagnostics,
) {
    let primary_key = if let Some(pk) = model.primary_key() {
        pk
    } else {
        return;
    };

    if primary_key.db_name().is_some() && !connector.supports_named_primary_keys() {
        diagnostics.push_error(DatamodelError::new_model_validation_error(
            "You defined a database name for the primary key on the model. This is not supported by the provider.",
            model.name(),
            model.ast_model().span,
        ));
    }

    if primary_key.fields().len() > 1 && !connector.supports_compound_ids() {
        return diagnostics.push_error(DatamodelError::new_model_validation_error(
            "The current connector does not support compound ids.",
            model.name(),
            primary_key.ast_attribute().span,
        ));
    }
}
