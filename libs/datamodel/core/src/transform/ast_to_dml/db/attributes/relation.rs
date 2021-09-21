use crate::{diagnostics::DatamodelError, transform::ast_to_dml::db::walkers::RelationFieldWalker};
use datamodel_connector::{Connector, ReferentialIntegrity};
use dml::relation_info::ReferentialAction;
use itertools::Itertools;

/// Validate that the arity of fields from `fields` is compatible with relation field arity.
pub(super) fn validate_relation_field_arity(field: RelationFieldWalker<'_, '_>, errors: &mut Vec<DatamodelError>) {
    if !field.ast_field().arity.is_required() {
        return;
    }

    if !field.referencing_fields().any(|field| field.is_optional()) {
        return;
    }

    errors.push(DatamodelError::new_validation_error(
        &format!(
            "The relation field `{}` uses the scalar fields {}. At least one of those fields is optional. Hence the relation field must be optional as well.",
            field.ast_field().name(),
            field.referencing_fields().map(|field| field.name()).join(", "),
        ),
        field.ast_field().span
    ));
}

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

/// Some connectors expect us to refer only unique fields from the foreign key.
pub(super) fn validate_references_unique_fields(
    field: RelationFieldWalker<'_, '_>,
    connector: &dyn Connector,
    errors: &mut Vec<DatamodelError>,
) {
    if field.referenced_fields().len() == 0 || !errors.is_empty() {
        return;
    }

    if connector.supports_relations_over_non_unique_criteria() {
        return;
    }

    let references_unique_criteria = field.related_model().unique_criterias().any(|criteria| {
        let mut criteria_field_names: Vec<_> = criteria.fields().map(|f| f.name()).collect();
        criteria_field_names.sort_unstable();

        let mut references_sorted: Vec<_> = field.referenced_fields().map(|f| f.name()).collect();
        references_sorted.sort_unstable();

        criteria_field_names == references_sorted
    });

    if references_unique_criteria {
        return;
    }

    errors.push(DatamodelError::new_validation_error(
        &format!(
            "The argument `references` must refer to a unique criteria in the related model `{}`. But it is referencing the following fields that are not a unique criteria: {}",
            field.related_model().ast_model().name(),
            field.referenced_fields().map(|f| f.ast_field().name()).join(", ")
        ),
        field.ast_field().span
    ));
}

/// Some connectors want the fields and references in the same order, and some
/// other connectors wants foreign keys to point to unique criterias.
pub(super) fn validate_referenced_fields_in_correct_order(
    field: RelationFieldWalker<'_, '_>,
    connector: &dyn Connector,
    errors: &mut Vec<DatamodelError>,
) {
    if field.referenced_fields().len() == 0 || !errors.is_empty() {
        return;
    }

    if connector.allows_relation_fields_in_arbitrary_order() || field.referenced_fields().len() <= 1 {
        return;
    }

    let reference_order_correct = field.related_model().unique_criterias().any(|criteria| {
        let criteria_fields = criteria.fields().map(|f| f.ast_field().name());

        if criteria_fields.len() != field.referenced_fields().len() {
            return false;
        }

        let references = field.referenced_fields().map(|f| f.ast_field().name());
        criteria_fields.zip(references).all(|(a, b)| a == b)
    });

    if reference_order_correct {
        return;
    }

    errors.push(DatamodelError::new_validation_error(
        &format!(
            "The argument `references` must refer to a unique criteria in the related model `{}` using the same order of fields. Please check the ordering in the following fields: `{}`.",
            field.related_model().ast_model().name(),
            field.referenced_fields().map(|f| f.ast_field().name()).join(", ")
        ),
        field.ast_field().span
    ));
}

/// The `fields` and `references` should hold the same number of fields.
pub(super) fn validate_same_length_in_referencing_and_referenced(
    field: RelationFieldWalker<'_, '_>,
    errors: &mut Vec<DatamodelError>,
) {
    if field.referenced_fields().len() == 0 || field.referencing_fields().len() == 0 {
        return;
    }

    if field.referenced_fields().len() == field.referencing_fields().len() {
        return;
    }

    errors.push(DatamodelError::new_attribute_validation_error(
        "You must specify the same number of fields in `fields` and `references`.",
        "relation",
        field.ast_field().span,
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
