use super::{
    constraint_namespace::ConstraintName,
    database_name::validate_db_name,
    names::{NameTaken, Names},
};
use crate::{
    ast,
    diagnostics::{DatamodelError, Diagnostics},
    transform::ast_to_dml::db::{
        walkers::{FieldWalker, ScalarFieldAttributeWalker, ScalarFieldWalker},
        ParserDatabase, ScalarFieldType,
    },
    Datasource,
};
use datamodel_connector::{
    connector_error::{ConnectorError, ErrorKind},
    Connector, ConnectorCapability,
};
use dml::scalars::ScalarType;
use itertools::Itertools;

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
pub(super) fn has_a_unique_default_constraint_name(
    field: ScalarFieldWalker<'_, '_>,
    names: &Names<'_>,
    diagnostics: &mut Diagnostics,
) {
    let name = match field.default_value().map(|w| w.constraint_name()) {
        Some(name) => name,
        None => return,
    };

    for violation in names
        .constraint_namespace
        .scope_violations(field.model().model_id(), ConstraintName::Default(name.as_ref()))
    {
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
    let connector_name = field
        .db
        .datasource()
        .map(|ds| ds.active_provider.clone())
        .unwrap_or_else(|| "Default".to_owned());
    let (scalar_type, (type_name, args, span)) = match (field.scalar_type(), field.raw_native_type()) {
        (Some(scalar_type), Some(raw)) => (scalar_type, raw),
        _ => return,
    };

    let constructor = if let Some(cons) = connector.find_native_type_constructor(type_name) {
        cons
    } else {
        diagnostics.push_error(DatamodelError::new_connector_error(
            &ConnectorError::from_kind(ErrorKind::NativeTypeNameUnknown {
                native_type: type_name.to_owned(),
                connector_name,
            })
            .to_string(),
            span,
        ));
        return;
    };

    let number_of_args = args.len();

    if number_of_args < constructor._number_of_args
        || ((number_of_args > constructor._number_of_args) && constructor._number_of_optional_args == 0)
    {
        diagnostics.push_error(DatamodelError::new_argument_count_missmatch_error(
            type_name,
            constructor._number_of_args,
            number_of_args,
            span,
        ));
        return;
    }

    if number_of_args > constructor._number_of_args + constructor._number_of_optional_args
        && constructor._number_of_optional_args > 0
    {
        diagnostics.push_error(DatamodelError::new_connector_error(
            &ConnectorError::from_kind(ErrorKind::OptionalArgumentCountMismatchError {
                native_type: type_name.to_owned(),
                optional_count: constructor._number_of_optional_args,
                given_count: number_of_args,
            })
            .to_string(),
            span,
        ));
        return;
    }

    // check for compatibility with scalar type
    if !constructor.prisma_types.contains(&scalar_type) {
        diagnostics.push_error(DatamodelError::new_connector_error(
            &ConnectorError::from_kind(ErrorKind::IncompatibleNativeType {
                native_type: type_name.to_owned(),
                field_type: scalar_type.to_string(),
                expected_types: constructor.prisma_types.iter().map(|s| s.to_string()).join(" or "),
            })
            .to_string(),
            span,
        ));
        return;
    }

    match connector.parse_native_type(type_name, args.to_owned()) {
        Ok(native_type) => {
            let mut errors = Vec::new();
            connector.validate_native_type_arguments(&native_type, &scalar_type, &mut errors);

            for error in errors {
                diagnostics.push_error(DatamodelError::ConnectorError {
                    message: error.to_string(),
                    span: field.ast_field().span,
                });
            }
        }
        Err(connector_error) => {
            diagnostics.push_error(DatamodelError::new_connector_error(&connector_error.to_string(), span));
        }
    };
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
            field.default_attribute().unwrap().span,
        ));
    }

    if has_db_name {
        validate_db_name(
            field.model().name(),
            field.default_attribute().unwrap(),
            default.and_then(|d| d.db_name()),
            connector,
            diagnostics,
            false,
        );
    }

    // Connector-specific validations.

    let scalar_type = if let Some(scalar_type) = field.scalar_type() {
        scalar_type
    } else {
        return;
    };

    let mut errors = Vec::new();
    if field.raw_native_type().is_none() {
        connector.validate_field_default_without_native_type(field.name(), &scalar_type, default, &mut errors);
    }

    for error in errors {
        diagnostics.push_error(DatamodelError::ConnectorError {
            message: error.to_string(),
            span: field.ast_field().span,
        });
    }
}

pub(super) fn validate_scalar_field_connector_specific(
    field: ScalarFieldWalker<'_, '_>,
    connector: &dyn Connector,
    diagnostics: &mut Diagnostics,
) {
    if matches!(field.scalar_field.r#type, ScalarFieldType::BuiltInScalar(t) if t.is_json())
        && !connector.supports_json()
    {
        diagnostics.push_error(DatamodelError::new_field_validation_error(
            &format!(
                "Field `{}` in model `{}` can't be of type Json. The current connector does not support the Json type.",
                field.name(),
                field.model().name()
            ),
            field.model().name(),
            field.name(),
            field.ast_field().span,
        ));
    }

    if field.ast_field().arity.is_list() && !connector.supports_scalar_lists() {
        diagnostics.push_error(DatamodelError::new_scalar_list_fields_are_not_supported(
            field.model().name(),
            field.name(),
            field.ast_field().span,
        ));
    }
}

pub(super) fn validate_unsupported_field_type(
    field: ScalarFieldWalker<'_, '_>,
    source: &Datasource,
    diagnostics: &mut Diagnostics,
) {
    use once_cell::sync::Lazy;
    use regex::Regex;

    static TYPE_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#"(?x)
    ^                           # beginning of the string
    (?P<prefix>[^(]+)           # a required prefix that is any character until the first opening brace
    (?:\((?P<params>.*?)\))?    # (optional) an opening parenthesis, a closing parenthesis and captured params in-between
    (?P<suffix>.+)?             # (optional) captured suffix after the params until the end of the string
    $                           # end of the string
    "#).unwrap()
    });

    let connector = source.active_connector;
    let (unsupported_lit, _) = if let ScalarFieldType::Unsupported = field.scalar_field.r#type {
        field.ast_field().field_type.as_unsupported().unwrap()
    } else {
        return;
    };

    if let Some(captures) = TYPE_REGEX.captures(unsupported_lit) {
        let prefix = captures.name("prefix").unwrap().as_str().trim();

        let params = captures.name("params");
        let args = match params {
            None => vec![],
            Some(params) => params.as_str().split(',').map(|s| s.trim().to_string()).collect(),
        };

        if let Ok(native_type) = connector.parse_native_type(prefix, args) {
            let prisma_type = connector.scalar_type_for_native_type(native_type.serialized_native_type.clone());

            let msg = format!(
                        "The type `Unsupported(\"{}\")` you specified in the type definition for the field `{}` is supported as a native type by Prisma. Please use the native type notation `{} @{}.{}` for full support.",
                        unsupported_lit, field.name(), prisma_type.to_string(), &source.name, native_type.render()
                    );

            diagnostics.push_error(DatamodelError::new_validation_error(msg, field.ast_field().span));
        }
    }
}
