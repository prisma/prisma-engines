use crate::{diagnostics::DatamodelError, transform::ast_to_dml::db::walkers::ModelWalker};
use datamodel_connector::Connector;
use diagnostics::Diagnostics;

pub(super) fn validate_auto_increment(
    model: ModelWalker<'_, '_>,
    connector: &dyn Connector,
    diagnostics: &mut Diagnostics,
) {
    let autoincrement_fields = || model.scalar_fields().filter(|f| f.is_autoincrement());

    if autoincrement_fields().next().is_none() {
        return; // just don't try to validate without autoincrement fields
    }

    let mut errors = Vec::new();

    // First check if the provider supports autoincrement at all. If yes, proceed with the detailed checks.
    if !connector.supports_auto_increment() {
        for field in autoincrement_fields() {
            // Add an error for all autoincrement fields on the model.
            errors.push(DatamodelError::new_attribute_validation_error(
                "The `autoincrement()` default value is used with a datasource that does not support it.",
                "default",
                field.default_attribute().unwrap().span,
            ));
        }

        return;
    }

    if !connector.supports_multiple_auto_increment() && autoincrement_fields().count() > 1 {
        errors.push(
            DatamodelError::new_attribute_validation_error(
            "The `autoincrement()` default value is used multiple times on this model even though the underlying datasource only supports one instance per table.",
            "default",
            model.ast_model().span,
        ))
    }

    // go over all fields
    for field in autoincrement_fields() {
        let field_is_indexed = || model.field_is_indexed_for_autoincrement(field.field_id());

        if !connector.supports_non_id_auto_increment() && !model.field_is_single_pk(field.field_id()) {
            errors.push(DatamodelError::new_attribute_validation_error(
                "The `autoincrement()` default value is used on a non-id field even though the datasource does not support this.",
                "default",
                field.ast_field().span
            ))
        }

        if !connector.supports_non_indexed_auto_increment() && !field_is_indexed() {
            errors.push(DatamodelError::new_attribute_validation_error(
                    "The `autoincrement()` default value is used on a non-indexed field even though the datasource does not support this.",
                    "default",
                    field.ast_field().span
                ))
        }
    }

    errors.into_iter().for_each(|err| diagnostics.push_error(err));
}
