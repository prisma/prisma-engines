use crate::{pretty_print::pretty_print, Span};
use std::{borrow::Cow, fmt};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct DatamodelError(DatamodelErrorKind);

impl fmt::Display for DatamodelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl From<DatamodelErrorKind> for DatamodelError {
    fn from(kind: DatamodelErrorKind) -> Self {
        DatamodelError(kind)
    }
}

/// Enum for different errors which can happen during parsing or validation.
///
/// For fancy printing, please use the `pretty_print_error` function.
// No format for this file, on purpose.
// Line breaks make the declarations very hard to read.
#[derive(Debug, Error, Clone, PartialEq)]
#[rustfmt::skip]
enum DatamodelErrorKind {
  #[error("Argument \"{}\" is missing.", argument_name)]
  ArgumentNotFound { argument_name: String, span: Span },

  #[error("Function \"{}\" takes {} arguments, but received {}.", function_name, required_count, given_count)]
  ArgumentCountMismatch { function_name: String, required_count: usize, given_count: usize, span: Span },

  #[error("Argument \"{}\" is missing in attribute \"@{}\".", argument_name, attribute_name)]
  AttributeArgumentNotFound { argument_name: String, attribute_name: String, span: Span },

  #[error("Native types are not supported with {} connector", connector_name)]
  NativeTypesNotSupported { connector_name: String, span: Span },

  #[error("Argument \"{}\" is missing in data source block \"{}\".", argument_name, source_name)]
  SourceArgumentNotFound { argument_name: String, source_name: String, span: Span },

  #[error("Argument \"{}\" is missing in generator block \"{}\".", argument_name, generator_name)]
  GeneratorArgumentNotFound { argument_name: String, generator_name: String, span: Span },

  #[error("Attribute \"@{}\" is defined twice.", attribute_name)]
  DuplicateAttributeError { attribute_name: String, span: Span },

  #[error("The model with database name \"{}\" could not be defined because another model with this name exists: \"{}\"", model_database_name, existing_model_name)]
  DuplicateModelDatabaseNameError { model_database_name: String, existing_model_name: String, span: Span },

  #[error("Invalid Native type {}.", native_type)]
  InvalidNativeType { native_type: String, span: Span },

  #[error(
      "Native type {} takes {} arguments, but received {}.",
      native_type,
      required_count,
      given_count
  )]
  NativeTypeArgumentCountMismatchError {
      native_type: String,
      required_count: usize,
      given_count: usize,
      span: Span,
  },

  #[error("\"{}\" is a reserved scalar type name and cannot be used.", type_name)]
  ReservedScalarTypeError { type_name: String, span: Span },

  #[error("The {} \"{}\" cannot be defined because a {} with that name already exists.", top_type, name, existing_top_type)]
  DuplicateTopError { name: String, top_type: String, existing_top_type: String, span: Span },

  // conf_block_name is pre-populated with "" in precheck.ts.
  #[error("Key \"{}\" is already defined in {}.", key_name, conf_block_name)]
  DuplicateConfigKeyError { conf_block_name: String, key_name: String, span: Span },

  #[error("Argument \"{}\" is already specified as unnamed argument.", arg_name)]
  DuplicateDefaultArgumentError { arg_name: String, span: Span },

  #[error("Argument \"{}\" is already specified.", arg_name)]
  DuplicateArgumentError { arg_name: String, span: Span },

  #[error("No such argument.")]
  UnusedArgumentError { arg_name: String, span: Span },

  #[error("Field \"{}\" is already defined on {} \"{}\".", field_name, container_type, model_name)]
  DuplicateFieldError { model_name: String, field_name: String, span: Span, container_type: &'static str },

  #[error("Field \"{}\" in model \"{}\" can't be a list. The current connector does not support lists of primitive types.", field_name, model_name)]
  ScalarListFieldsAreNotSupported { model_name: String, field_name: String, span: Span },

  #[error("Value \"{}\" is already defined on enum \"{}\".", value_name, enum_name)]
  DuplicateEnumValueError { enum_name: String, value_name: String, span: Span },

  #[error("Attribute not known: \"@{}\".", attribute_name)]
  AttributeNotKnownError { attribute_name: String, span: Span },

  #[error("Property not known: \"{}\".", property_name)]
  PropertyNotKnownError { property_name: String, span: Span },

  #[error("Datasource provider not known: \"{}\".", provider)]
  DatasourceProviderNotKnownError { provider: String, span: Span },

  #[error("shadowDatabaseUrl is the same as url for datasource \"{}\". Please specify a different database as shadow database.", source_name)]
  ShadowDatabaseUrlIsSameAsMainUrl { source_name: String, span: Span },

  #[error("The preview feature \"{}\" is not known. Expected one of: {}", preview_feature, expected_preview_features)]
  PreviewFeatureNotKnownError { preview_feature: String, expected_preview_features: String, span: Span },

  #[error("\"{}\" is not a valid value for {}.", raw_value, literal_type)]
  LiteralParseError { literal_type: String, raw_value: String, span: Span },

  #[error("Type \"{}\" is neither a built-in type, nor refers to another model, custom type, or enum.", type_name)]
  TypeNotFoundError { type_name: String, span: Span },

  #[error("Type \"{}\" is not a built-in type.", type_name)]
  ScalarTypeNotFoundError { type_name: String, span: Span },

  #[error("Unexpected token. Expected one of: {}", expected_str)]
  ParserError { expected_str: String, span: Span },

  #[error("Environment variable not found: {}.", var_name)]
  EnvironmentFunctionalEvaluationError { var_name: String, span: Span },

  #[error("Expected a {} value, but received {} value `{}`.", expected_type, received_type, raw)]
  TypeMismatchError { expected_type: String, received_type: String, raw: String, span: Span },

  #[error("Expected a {} value, but failed while parsing \"{}\": {}.", expected_type, raw, parser_error)]
  ValueParserError { expected_type: String, parser_error: String, raw: String, span: Span },

  #[error("Error validating model \"{}\": {}", model_name, message)]
  ModelValidationError { message: String, model_name: String, span: Span  },

  #[error("Error validating composite type \"{}\": {}", composite_type_name, message)]
  CompositeTypeValidationError { message: String, composite_type_name: String, span: Span  },

  #[error("Error validating field `{}` in {} `{}`: {}", field, container_type, container_name, message)]
  FieldValidationError { message: String, container_name: String, field: String, span: Span, container_type: &'static str },

  #[error("Error validating datasource `{datasource}`: {message}")]
  SourceValidationError { message: String, datasource: String, span: Span },

  #[error("Error validating enum `{}`: {}", enum_name, message)]
  EnumValidationError { message: String, enum_name: String, span: Span },

  #[error("Error validating: {}", message)]
  ValidationError { message: String, span: Span },

  #[error("{}", message)]
  Raw { message: Cow<'static, str>, span: Span },

}

impl DatamodelError {
    pub fn new(message: Cow<'static, str>, span: Span) -> Self {
        DatamodelError(DatamodelErrorKind::Raw { message, span })
    }

    pub fn new_literal_parser_error(literal_type: &str, raw_value: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::LiteralParseError {
            literal_type: String::from(literal_type),
            raw_value: String::from(raw_value),
            span,
        }
        .into()
    }

    pub fn new_argument_not_found_error(argument_name: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::ArgumentNotFound {
            argument_name: String::from(argument_name),
            span,
        }
        .into()
    }

    pub fn new_argument_count_mismatch_error(
        function_name: &str,
        required_count: usize,
        given_count: usize,
        span: Span,
    ) -> DatamodelError {
        DatamodelErrorKind::ArgumentCountMismatch {
            function_name: String::from(function_name),
            required_count,
            given_count,
            span,
        }
        .into()
    }

    pub fn new_attribute_argument_not_found_error(
        argument_name: &str,
        attribute_name: &str,
        span: Span,
    ) -> DatamodelError {
        DatamodelErrorKind::AttributeArgumentNotFound {
            argument_name: String::from(argument_name),
            attribute_name: String::from(attribute_name),
            span,
        }
        .into()
    }

    pub fn new_source_argument_not_found_error(argument_name: &str, source_name: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::SourceArgumentNotFound {
            argument_name: String::from(argument_name),
            source_name: String::from(source_name),
            span,
        }
        .into()
    }

    pub fn new_generator_argument_not_found_error(
        argument_name: &str,
        generator_name: &str,
        span: Span,
    ) -> DatamodelError {
        DatamodelErrorKind::GeneratorArgumentNotFound {
            argument_name: String::from(argument_name),
            generator_name: String::from(generator_name),
            span,
        }
        .into()
    }

    pub fn new_attribute_validation_error(message: &str, attribute_name: &str, span: Span) -> DatamodelError {
        let msg = format!("Error parsing attribute \"{attribute_name}\": {message}");
        Self::new(msg.into(), span)
    }

    pub fn new_duplicate_attribute_error(attribute_name: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::DuplicateAttributeError {
            attribute_name: String::from(attribute_name),
            span,
        }
        .into()
    }

    pub fn new_incompatible_native_type(
        native_type: &str,
        field_type: &str,
        expected_types: &str,
        span: Span,
    ) -> DatamodelError {
        let msg = format!(
            "Native type {} is not compatible with declared field type {}, expected field type {}.",
            native_type, field_type, expected_types
        );
        DatamodelError::new(msg.into(), span)
    }

    pub fn new_invalid_native_type_argument(
        native_type: &str,
        got: &str,
        expected: &str,
        span: Span,
    ) -> DatamodelError {
        let msg = format!(
            "Invalid argument for type {}: {}. Allowed values: {}.",
            native_type, got, expected
        );
        DatamodelError::new(msg.into(), span)
    }

    pub fn new_invalid_prefix_for_native_types(
        given_prefix: &str,
        expected_prefix: &str,
        suggestion: &str,
        span: Span,
    ) -> DatamodelError {
        let msg =  format!("The prefix {} is invalid. It must be equal to the name of an existing datasource e.g. {}. Did you mean to use {}?", given_prefix, expected_prefix, suggestion);
        DatamodelError::new(msg.into(), span)
    }

    pub fn new_native_types_not_supported(connector_name: String, span: Span) -> DatamodelError {
        DatamodelErrorKind::NativeTypesNotSupported { connector_name, span }.into()
    }

    pub fn new_reserved_scalar_type_error(type_name: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::ReservedScalarTypeError {
            type_name: String::from(type_name),
            span,
        }
        .into()
    }

    pub fn new_duplicate_model_database_name_error(
        model_database_name: String,
        existing_model_name: String,
        span: Span,
    ) -> DatamodelError {
        DatamodelErrorKind::DuplicateModelDatabaseNameError {
            model_database_name,
            existing_model_name,
            span,
        }
        .into()
    }

    pub fn new_duplicate_top_error(name: &str, top_type: &str, existing_top_type: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::DuplicateTopError {
            name: String::from(name),
            top_type: String::from(top_type),
            existing_top_type: String::from(existing_top_type),
            span,
        }
        .into()
    }

    pub fn new_duplicate_config_key_error(conf_block_name: &str, key_name: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::DuplicateConfigKeyError {
            conf_block_name: String::from(conf_block_name),
            key_name: String::from(key_name),
            span,
        }
        .into()
    }

    pub fn new_duplicate_argument_error(arg_name: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::DuplicateArgumentError {
            arg_name: String::from(arg_name),
            span,
        }
        .into()
    }

    pub fn new_unused_argument_error(arg_name: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::UnusedArgumentError {
            arg_name: String::from(arg_name),
            span,
        }
        .into()
    }

    pub fn new_duplicate_default_argument_error(arg_name: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::DuplicateDefaultArgumentError {
            arg_name: String::from(arg_name),
            span,
        }
        .into()
    }

    pub fn new_duplicate_enum_value_error(enum_name: &str, value_name: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::DuplicateEnumValueError {
            enum_name: String::from(enum_name),
            value_name: String::from(value_name),
            span,
        }
        .into()
    }

    pub fn new_composite_type_duplicate_field_error(type_name: &str, field_name: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::DuplicateFieldError {
            container_type: "composite type",
            model_name: String::from(type_name),
            field_name: String::from(field_name),
            span,
        }
        .into()
    }

    pub fn new_duplicate_field_error(model_name: &str, field_name: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::DuplicateFieldError {
            container_type: "model",
            model_name: String::from(model_name),
            field_name: String::from(field_name),
            span,
        }
        .into()
    }

    pub fn new_scalar_list_fields_are_not_supported(model_name: &str, field_name: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::ScalarListFieldsAreNotSupported {
            model_name: String::from(model_name),
            field_name: String::from(field_name),
            span,
        }
        .into()
    }

    pub fn new_model_validation_error(message: &str, model_name: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::ModelValidationError {
            message: String::from(message),
            model_name: String::from(model_name),
            span,
        }
        .into()
    }

    pub fn new_composite_type_validation_error(
        message: String,
        composite_type_name: String,
        span: Span,
    ) -> DatamodelError {
        DatamodelErrorKind::CompositeTypeValidationError {
            message,
            composite_type_name,
            span,
        }
        .into()
    }

    pub fn new_enum_validation_error(message: String, enum_name: String, span: Span) -> DatamodelError {
        DatamodelErrorKind::EnumValidationError {
            message,
            enum_name,
            span,
        }
        .into()
    }

    pub fn new_composite_type_field_validation_error(
        message: &str,
        composite_type_name: &str,
        field: &str,
        span: Span,
    ) -> DatamodelError {
        DatamodelErrorKind::FieldValidationError {
            message: message.to_owned(),
            container_name: composite_type_name.to_owned(),
            container_type: "composite type",
            field: field.to_owned(),
            span,
        }
        .into()
    }

    pub fn new_field_validation_error(message: &str, model: &str, field: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::FieldValidationError {
            message: message.to_owned(),
            container_name: model.to_owned(),
            container_type: "model",
            field: field.to_owned(),
            span,
        }
        .into()
    }

    pub fn new_source_validation_error(message: &str, source: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::SourceValidationError {
            message: message.to_owned(),
            datasource: source.to_owned(),
            span,
        }
        .into()
    }

    pub fn new_validation_error(message: String, span: Span) -> DatamodelError {
        DatamodelErrorKind::ValidationError { message, span }.into()
    }

    pub fn new_legacy_parser_error(message: impl Into<Cow<'static, str>>, span: Span) -> DatamodelError {
        Self::new(message.into(), span)
    }

    pub fn new_optional_argument_count_mismatch(
        native_type: &str,
        optional_count: usize,
        given_count: usize,
        span: Span,
    ) -> DatamodelError {
        let msg = format!(
            "Native type {} takes {} optional arguments, but received {}.",
            native_type, optional_count, given_count
        );
        DatamodelError::new(msg.into(), span)
    }

    pub fn new_parser_error(expected_str: String, span: Span) -> DatamodelError {
        DatamodelErrorKind::ParserError { expected_str, span }.into()
    }

    pub fn new_functional_evaluation_error(message: impl Into<Cow<'static, str>>, span: Span) -> DatamodelError {
        Self::new(message.into(), span)
    }

    pub fn new_environment_functional_evaluation_error(var_name: String, span: Span) -> DatamodelError {
        DatamodelErrorKind::EnvironmentFunctionalEvaluationError { var_name, span }.into()
    }

    pub fn new_type_not_found_error(type_name: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::TypeNotFoundError {
            type_name: String::from(type_name),
            span,
        }
        .into()
    }

    pub fn new_scalar_type_not_found_error(type_name: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::ScalarTypeNotFoundError {
            type_name: String::from(type_name),
            span,
        }
        .into()
    }

    pub fn new_attribute_not_known_error(attribute_name: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::AttributeNotKnownError {
            attribute_name: String::from(attribute_name),
            span,
        }
        .into()
    }

    pub fn new_property_not_known_error(property_name: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::PropertyNotKnownError {
            property_name: String::from(property_name),
            span,
        }
        .into()
    }

    pub fn new_default_unknown_function(function_name: &str, span: Span) -> DatamodelError {
        DatamodelError::new(format!(
                "Unknown function in @default(): `{function_name}` is not known. You can read about the available functions here: https://pris.ly/d/attribute-functions"
            ).into(),
            span
        )
    }

    pub fn new_invalid_model_error(msg: &str, span: Span) -> DatamodelError {
        DatamodelError::new(format!("Invalid model: {}", msg).into(), span)
    }

    pub fn new_datasource_provider_not_known_error(provider: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::DatasourceProviderNotKnownError {
            provider: String::from(provider),
            span,
        }
        .into()
    }

    pub fn new_shadow_database_is_same_as_main_url_error(source_name: String, span: Span) -> DatamodelError {
        DatamodelErrorKind::ShadowDatabaseUrlIsSameAsMainUrl { source_name, span }.into()
    }

    pub fn new_preview_feature_not_known_error(
        preview_feature: &str,
        expected_preview_features: String,
        span: Span,
    ) -> DatamodelError {
        DatamodelErrorKind::PreviewFeatureNotKnownError {
            preview_feature: String::from(preview_feature),
            expected_preview_features,
            span,
        }
        .into()
    }

    pub fn new_value_parser_error(expected_type: &str, parser_error: &str, raw: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::ValueParserError {
            expected_type: String::from(expected_type),
            parser_error: String::from(parser_error),
            raw: String::from(raw),
            span,
        }
        .into()
    }

    pub fn new_native_type_argument_count_mismatch_error(
        native_type: &str,
        required_count: usize,
        given_count: usize,
        span: Span,
    ) -> DatamodelError {
        DatamodelErrorKind::NativeTypeArgumentCountMismatchError {
            native_type: String::from(native_type),
            required_count,
            given_count,
            span,
        }
        .into()
    }

    pub fn new_native_type_name_unknown(connector_name: &str, native_type: &str, span: Span) -> DatamodelError {
        let msg = format!(
            "Native type {} is not supported for {} connector.",
            native_type, connector_name
        );
        DatamodelError::new(msg.into(), span)
    }

    pub fn new_native_type_parser_error(native_type: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::InvalidNativeType {
            native_type: String::from(native_type),
            span,
        }
        .into()
    }

    pub fn new_type_mismatch_error(expected_type: &str, received_type: &str, raw: &str, span: Span) -> DatamodelError {
        DatamodelErrorKind::TypeMismatchError {
            expected_type: String::from(expected_type),
            received_type: String::from(received_type),
            raw: String::from(raw),
            span,
        }
        .into()
    }

    pub fn span(&self) -> Span {
        match &self.0 {
            DatamodelErrorKind::ArgumentNotFound { span, .. } => *span,
            DatamodelErrorKind::AttributeArgumentNotFound { span, .. } => *span,
            DatamodelErrorKind::ArgumentCountMismatch { span, .. } => *span,
            DatamodelErrorKind::SourceArgumentNotFound { span, .. } => *span,
            DatamodelErrorKind::GeneratorArgumentNotFound { span, .. } => *span,
            DatamodelErrorKind::AttributeNotKnownError { span, .. } => *span,
            DatamodelErrorKind::ReservedScalarTypeError { span, .. } => *span,
            DatamodelErrorKind::DatasourceProviderNotKnownError { span, .. } => *span,
            DatamodelErrorKind::LiteralParseError { span, .. } => *span,
            DatamodelErrorKind::NativeTypeArgumentCountMismatchError { span, .. } => *span,
            DatamodelErrorKind::TypeNotFoundError { span, .. } => *span,
            DatamodelErrorKind::ScalarTypeNotFoundError { span, .. } => *span,
            DatamodelErrorKind::ParserError { span, .. } => *span,
            DatamodelErrorKind::EnvironmentFunctionalEvaluationError { span, .. } => *span,
            DatamodelErrorKind::TypeMismatchError { span, .. } => *span,
            DatamodelErrorKind::ValueParserError { span, .. } => *span,
            DatamodelErrorKind::ValidationError { span, .. } => *span,
            DatamodelErrorKind::ModelValidationError { span, .. } => *span,
            DatamodelErrorKind::DuplicateAttributeError { span, .. } => *span,
            DatamodelErrorKind::DuplicateConfigKeyError { span, .. } => *span,
            DatamodelErrorKind::DuplicateTopError { span, .. } => *span,
            DatamodelErrorKind::DuplicateFieldError { span, .. } => *span,
            DatamodelErrorKind::DuplicateEnumValueError { span, .. } => *span,
            DatamodelErrorKind::DuplicateArgumentError { span, .. } => *span,
            DatamodelErrorKind::DuplicateModelDatabaseNameError { span, .. } => *span,
            DatamodelErrorKind::DuplicateDefaultArgumentError { span, .. } => *span,
            DatamodelErrorKind::UnusedArgumentError { span, .. } => *span,
            DatamodelErrorKind::ScalarListFieldsAreNotSupported { span, .. } => *span,
            DatamodelErrorKind::FieldValidationError { span, .. } => *span,
            DatamodelErrorKind::SourceValidationError { span, .. } => *span,
            DatamodelErrorKind::EnumValidationError { span, .. } => *span,
            DatamodelErrorKind::Raw { span, .. } => *span,
            DatamodelErrorKind::PreviewFeatureNotKnownError { span, .. } => *span,
            DatamodelErrorKind::ShadowDatabaseUrlIsSameAsMainUrl { span, .. } => *span,
            DatamodelErrorKind::CompositeTypeValidationError { span, .. } => *span,
            DatamodelErrorKind::PropertyNotKnownError { span, .. } => *span,
            DatamodelErrorKind::InvalidNativeType { span, .. } => *span,
            DatamodelErrorKind::NativeTypesNotSupported { span, .. } => *span,
        }
    }

    pub fn description(&self) -> String {
        self.to_string()
    }

    pub fn pretty_print(&self, f: &mut dyn std::io::Write, file_name: &str, text: &str) -> std::io::Result<()> {
        pretty_print(f, file_name, text, self.span(), self.description().as_str())
    }
}
