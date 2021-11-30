use datamodel_connector::{Connector, ConnectorCapability};
use diagnostics::Span;
use dml::scalars::ScalarType;

use super::names::{NameTaken, Names};
use crate::{
    ast,
    diagnostics::{DatamodelError, Diagnostics},
    transform::ast_to_dml::db::{
        walkers::{FieldWalker, ScalarFieldAttributeWalker, ScalarFieldWalker},
        ConstraintName, ParserDatabase,
    },
};

pub(super) fn validate_client_name(field: FieldWalker<'_, '_>, names: &Names<'_>, diagnostics: &mut Diagnostics) {
    let model = field.model();

    for taken in names.name_taken(model.model_id(), field.name()).into_iter() {
        match taken {
            NameTaken::Index => {
                let message = format!(
                    "The custom name `{}` specified for the `@@index` attribute is already used as a name for a field. Please choose a different name.",
                    field.name()
                );

                let error = DatamodelError::new_model_validation_error(&message, model.name(), model.ast_model().span);
                diagnostics.push_error(error);
            }
            NameTaken::Unique => {
                let message = format!(
                    "The custom name `{}` specified for the `@@unique` attribute is already used as a name for a field. Please choose a different name.",
                    field.name()
                );

                let error = DatamodelError::new_model_validation_error(&message, model.name(), model.ast_model().span);
                diagnostics.push_error(error);
            }
            NameTaken::PrimaryKey => {
                let message = format!(
                    "The custom name `{}` specified for the `@@id` attribute is already used as a name for a field. Please choose a different name.",
                    field.name()
                );

                let error = DatamodelError::new_model_validation_error(&message, model.name(), model.ast_model().span);
                diagnostics.push_error(error);
            }
        }
    }
}

/// Some databases use constraints for default values, with a name that can be unique in a certain
/// namespace. Validates the field default constraint against name clases.
pub(crate) fn has_a_unique_default_constraint_name(
    db: &ParserDatabase<'_>,
    field: ScalarFieldWalker<'_, '_>,
    diagnostics: &mut Diagnostics,
) {
    let name = match field.default_value().map(|w| w.constraint_name()) {
        Some(name) => name,
        None => return,
    };

    for violation in db.scope_violations(field.model().model_id(), ConstraintName::Default(name.as_ref())) {
        let message = format!(
            "The given constraint name `{}` has to be unique in the following namespace: {}. Please provide a different name using the `map` argument.",
            name,
            violation.description(field.model().name()),
        );

        let span = field
            .ast_field()
            .span_for_argument("default", "map")
            .unwrap_or(field.ast_field().span);

        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            &message, "default", span,
        ));
    }
}

/// The length prefix can be used with strings and byte columns.
pub(crate) fn validate_length_used_with_correct_types(
    db: &ParserDatabase<'_>,
    attr: ScalarFieldAttributeWalker<'_, '_>,
    attribute: (&str, ast::Span),
    diagnostics: &mut Diagnostics,
) {
    if !db
        .active_connector()
        .has_capability(ConnectorCapability::IndexColumnLengthPrefixing)
    {
        return;
    }

    if attr.length().is_none() {
        return;
    }

    if let Some(r#type) = attr.as_scalar_field().attributes().r#type.as_builtin_scalar() {
        if [ScalarType::String, ScalarType::Bytes].iter().any(|t| t == &r#type) {
            return;
        }
    };

    let message = "The length argument is only allowed with field types `String` or `Bytes`.";

    diagnostics.push_error(DatamodelError::new_attribute_validation_error(
        message,
        attribute.0,
        attribute.1,
    ));
}

pub(super) fn validate_native_type_arguments(field: ScalarFieldWalker<'_, '_>, diagnostics: &mut Diagnostics) {
    let connector = field.db.active_connector();
    let (scalar_type, native_type) = match (field.scalar_type(), field.native_type_instance()) {
        (Some(scalar_type), Some(native_type)) => (scalar_type, native_type),
        _ => return,
    };

    let mut errors = Vec::new();
    connector.validate_native_type_arguments(&native_type, &scalar_type, &mut errors);

    for error in errors {
        diagnostics.push_error(DatamodelError::ConnectorError {
            message: error.to_string(),
            span: field.ast_field().span,
        });
    }
}

pub(super) fn validate_default(
    field: ScalarFieldWalker<'_, '_>,
    connector: &dyn Connector,
    diagnostics: &mut Diagnostics,
) {
    // Named defaults.

    let default = field.default_value().map(|d| d.default());
    let has_db_name = default.map(|d| d.db_name().is_some()).unwrap_or_default();

    if has_db_name && !connector.supports_named_default_values() {
        diagnostics.push_error(DatamodelError::new_attribute_validation_error(
            "You defined a database name for the default value of a field on the model. This is not supported by the provider.",
            "default",
            field.ast_field().span_for_attribute("default").unwrap_or(Span::empty()),
        ));
    }

    // Connector-specific validations.

    let (scalar_type, native_type) = match (field.scalar_type(), field.native_type_instance()) {
        (Some(scalar_type), native_type) => (scalar_type, native_type),
        _ => return,
    };

    let mut errors = Vec::new();
    connector.validate_field_default(field.name(), &scalar_type, native_type.as_ref(), default, &mut errors);

    for error in errors {
        diagnostics.push_error(DatamodelError::ConnectorError {
            message: error.to_string(),
            span: field.ast_field().span,
        });
    }
}
