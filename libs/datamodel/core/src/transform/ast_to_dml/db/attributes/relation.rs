use crate::{diagnostics::DatamodelError, transform::ast_to_dml::db::walkers::RelationFieldWalker};
use datamodel_connector::{Connector, ReferentialIntegrity};
use dml::relation_info::ReferentialAction;
use itertools::Itertools;

/// Validates usage of `onUpdate` with the `referentialIntegrity` set to
/// `prisma`.
///
/// This is temporary to the point until Query Engine supports `onUpdate`
/// actions on emulations.
pub(super) fn validate_on_update_without_foreign_keys(
    field: RelationFieldWalker<'_, '_>,
    referential_integrity: ReferentialIntegrity,
    errors: &mut Vec<DatamodelError>,
) {
    if referential_integrity.uses_foreign_keys() {
        return;
    }

    if field
        .attributes()
        .on_update
        .filter(|act| *act != ReferentialAction::NoAction)
        .is_none()
    {
        return;
    }

    let ast_field = field.ast_field();

    let span = ast_field
        .span_for_argument("relation", "onUpdate")
        .unwrap_or(ast_field.span);

    errors.push(DatamodelError::new_validation_error(
        "Referential actions other than `NoAction` will not work for `onUpdate` without foreign keys. Please follow the issue: https://github.com/prisma/prisma/issues/9014",
        span
    ));
}

/// Validates if the related model for the relation is ignored.
pub(super) fn validate_ignored_related_model(field: RelationFieldWalker<'_, '_>, errors: &mut Vec<DatamodelError>) {
    let related_model = field.related_model();
    let model = field.model();

    if !related_model.attributes().is_ignored || field.attributes().is_ignored || model.attributes().is_ignored {
        return;
    }

    let ast_model = model.ast_model();
    let ast_related_model = related_model.ast_model();
    let ast_field = field.ast_field();

    let message = format!(
        "The relation field `{}` on Model `{}` must specify the `@ignore` attribute, because the model {} it is pointing to is marked ignored.",
        ast_field.name(), ast_model.name(), ast_related_model.name()
    );

    errors.push(DatamodelError::new_attribute_validation_error(
        &message,
        "ignore",
        ast_field.span,
    ));
}

/// Does the connector support the given referential actions.
pub(super) fn validate_referential_actions(
    field: RelationFieldWalker<'_, '_>,
    connector: &dyn Connector,
    errors: &mut Vec<DatamodelError>,
) {
    let msg = |action| {
        let allowed_values = connector
            .referential_actions()
            .iter()
            .map(|f| format!("`{}`", f))
            .join(", ");

        format!(
            "Invalid referential action: `{}`. Allowed values: ({})",
            action, allowed_values,
        )
    };

    if let Some(on_delete) = field.attributes().on_delete {
        if !connector.supports_referential_action(on_delete) {
            let span = field
                .ast_field()
                .span_for_argument("relation", "onDelete")
                .unwrap_or_else(|| field.ast_field().span);

            errors.push(DatamodelError::new_validation_error(&msg(on_delete), span));
        }
    }

    if let Some(on_update) = field.attributes().on_update {
        if !connector.supports_referential_action(on_update) {
            let span = field
                .ast_field()
                .span_for_argument("relation", "onUpdate")
                .unwrap_or_else(|| field.ast_field().span);

            errors.push(DatamodelError::new_validation_error(&msg(on_update), span));
        }
    }
}
