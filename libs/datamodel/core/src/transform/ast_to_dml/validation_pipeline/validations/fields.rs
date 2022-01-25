use super::{
    constraint_namespace::ConstraintName,
    database_name::validate_db_name,
    default_value,
    names::{NameTaken, Names},
};
use crate::{
    ast,
    diagnostics::DatamodelError,
    transform::ast_to_dml::{
        db::{
            walkers::{FieldWalker, ScalarFieldAttributeWalker, ScalarFieldWalker},
            ScalarFieldType, ScalarType,
        },
        validation_pipeline::context::Context,
    },
};
use datamodel_connector::{
    connector_error::{ConnectorError, ErrorKind},
    walker_ext_traits::*,
    ConnectorCapability,
};

pub(super) fn validate_client_name(field: FieldWalker<'_, '_>, names: &Names<'_, '_>, ctx: &mut Context<'_>) {
    let model = field.model();

    for taken in names.name_taken(model.model_id(), field.name()).into_iter() {
        match taken {
            NameTaken::Index => {
                let message = format!(
                    "The custom name `{}` specified for the `@@index` attribute is already used as a name for a field. Please choose a different name.",
                    field.name()
                );

                let error = DatamodelError::new_model_validation_error(&message, model.name(), model.ast_model().span);
                ctx.push_error(error);
            }
            NameTaken::Unique => {
                let message = format!(
                    "The custom name `{}` specified for the `@@unique` attribute is already used as a name for a field. Please choose a different name.",
                    field.name()
                );

                let error = DatamodelError::new_model_validation_error(&message, model.name(), model.ast_model().span);
                ctx.push_error(error);
            }
            NameTaken::PrimaryKey => {
                let message = format!(
                    "The custom name `{}` specified for the `@@id` attribute is already used as a name for a field. Please choose a different name.",
                    field.name()
                );

                let error = DatamodelError::new_model_validation_error(&message, model.name(), model.ast_model().span);
                ctx.push_error(error);
            }
        }
    }
}

/// Some databases use constraints for default values, with a name that can be unique in a certain
/// namespace. Validates the field default constraint against name clases.
pub(super) fn has_a_unique_default_constraint_name(
    field: ScalarFieldWalker<'_, '_>,
    names: &Names<'_, '_>,
    ctx: &mut Context<'_>,
) {
    let name = match field.default_value().map(|w| w.constraint_name(ctx.connector)) {
        Some(name) => name,
        None => return,
    };

    for violation in names
        .constraint_namespace
        .constraint_name_scope_violations(field.model().model_id(), ConstraintName::Default(name.as_ref()))
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

        ctx.push_error(DatamodelError::new_attribute_validation_error(
            &message, "default", span,
        ));
    }
}

/// The length prefix can be used with strings and byte columns.
pub(crate) fn validate_length_used_with_correct_types(
    attr: ScalarFieldAttributeWalker<'_, '_>,
    attribute: (&str, ast::Span),
    ctx: &mut Context<'_>,
) {
    if !ctx
        .connector
        .has_capability(ConnectorCapability::IndexColumnLengthPrefixing)
    {
        return;
    }

    if attr.length().is_none() {
        return;
    }

    if let Some(r#type) = attr.as_scalar_field().scalar_field_type().as_builtin_scalar() {
        if [ScalarType::String, ScalarType::Bytes].iter().any(|t| t == &r#type) {
            return;
        }
    };

    let message = "The length argument is only allowed with field types `String` or `Bytes`.";

    ctx.push_error(DatamodelError::new_attribute_validation_error(
        message,
        attribute.0,
        attribute.1,
    ));
}

pub(super) fn validate_native_type_arguments(field: ScalarFieldWalker<'_, '_>, ctx: &mut Context<'_>) {
    let connector_name = ctx
        .datasource
        .map(|ds| ds.active_provider.clone())
        .unwrap_or_else(|| "Default".to_owned());
    let (scalar_type, (attr_scope, type_name, args, span)) = match (field.scalar_type(), field.raw_native_type()) {
        (Some(scalar_type), Some(raw)) => (scalar_type, raw),
        _ => return,
    };

    // Validate that the attribute is scoped with the right datasource name.
    if let Some(datasource) = ctx.datasource {
        if datasource.name != attr_scope {
            ctx.push_error(DatamodelError::new_connector_error(
                &ConnectorError::from_kind(ErrorKind::InvalidPrefixForNativeTypes {
                    given_prefix: attr_scope.to_owned(),
                    expected_prefix: datasource.name.clone(),
                    suggestion: [datasource.name.as_str(), type_name].join("."),
                })
                .to_string(),
                span,
            ));
        }
    }

    let constructor = if let Some(cons) = ctx.connector.find_native_type_constructor(type_name) {
        cons
    } else {
        ctx.push_error(DatamodelError::new_connector_error(
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
        ctx.push_error(DatamodelError::new_argument_count_missmatch_error(
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
        ctx.push_error(DatamodelError::new_connector_error(
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
        ctx.push_error(DatamodelError::new_connector_error(
            &ConnectorError::from_kind(ErrorKind::IncompatibleNativeType {
                native_type: type_name.to_owned(),
                field_type: scalar_type.as_str(),
                expected_types: constructor
                    .prisma_types
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(" or "),
            })
            .to_string(),
            span,
        ));
        return;
    }

    match ctx.connector.parse_native_type(type_name, args.to_owned()) {
        Ok(native_type) => {
            let mut errors = Vec::new();
            ctx.connector
                .validate_native_type_arguments(&native_type, &scalar_type, &mut errors);

            for error in errors {
                ctx.push_error(DatamodelError::ConnectorError {
                    message: error.to_string(),
                    span: field.ast_field().span,
                });
            }
        }
        Err(connector_error) => {
            ctx.push_error(DatamodelError::new_connector_error(&connector_error.to_string(), span));
        }
    };
}

/// Validates the @default attribute of a model scalar field
pub(super) fn validate_default_value(field: ScalarFieldWalker<'_, '_>, ctx: &mut Context<'_>) {
    let model_name = field.model().name();
    let default_mapped_name = field.default_value().and_then(|d| d.mapped_name());
    let default_attribute = field.default_attribute();

    // Named defaults.
    if default_mapped_name.is_some() && !ctx.connector.supports_named_default_values() {
        ctx.push_error(DatamodelError::new_attribute_validation_error(
            "You defined a database name for the default value of a field on the model. This is not supported by the provider.",
            "default",
            default_attribute.unwrap().span,
        ));
    }

    if default_mapped_name.is_some() {
        validate_db_name(model_name, default_attribute.unwrap(), default_mapped_name, ctx, false);
    }

    let default_value = field.default_value().map(|d| d.value());
    let scalar_type = field.scalar_type();

    default_value::validate_default_value(default_value, scalar_type, ctx);
}

pub(super) fn validate_scalar_field_connector_specific(field: ScalarFieldWalker<'_, '_>, ctx: &mut Context<'_>) {
    if matches!(
        field.scalar_field_type(),
        ScalarFieldType::BuiltInScalar(ScalarType::Json)
    ) {
        if !ctx.connector.supports_json() {
            ctx.push_error(DatamodelError::new_field_validation_error(
                &format!(
                    "Field `{}` in model `{}` can't be of type Json. The current connector does not support the Json type.",
                    field.name(),
                    field.model().name(),
                ),
                field.model().name(),
                field.name(),
                field.ast_field().span,
            ));
        }

        if field.ast_field().arity.is_list() && !ctx.connector.supports_json_lists() {
            ctx.push_error(DatamodelError::new_field_validation_error(
                &format!(
                    "Field `{}` in model `{}` can't be of type Json[]. The current connector does not support the Json List type.",
                    field.name(),
                    field.model().name()
                ),
                field.model().name(),
                field.name(),
                field.ast_field().span,
            ));
        }
    }

    if field.ast_field().arity.is_list() && !ctx.connector.supports_scalar_lists() {
        ctx.push_error(DatamodelError::new_scalar_list_fields_are_not_supported(
            field.model().name(),
            field.name(),
            field.ast_field().span,
        ));
    }
}

pub(super) fn validate_unsupported_field_type(field: ScalarFieldWalker<'_, '_>, ctx: &mut Context<'_>) {
    use once_cell::sync::Lazy;
    use regex::Regex;

    let source = if let Some(s) = ctx.datasource { s } else { return };

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
    let (unsupported_lit, _) = if let ScalarFieldType::Unsupported = field.scalar_field_type() {
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
                        unsupported_lit, field.name(), prisma_type.as_str(), &source.name, native_type
                    );

            ctx.push_error(DatamodelError::new_validation_error(msg, field.ast_field().span));
        }
    }
}
