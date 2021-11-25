use crate::ast::Span;
use crate::diagnostics::helper::pretty_print;
use thiserror::Error;

/// Enum for different errors which can happen during parsing or validation.
///
/// For fancy printing, please use the `pretty_print_error` function.
// No format for this file, on purpose.
// Line breaks make the declarations very hard to read.
#[derive(Debug, Error, Clone, PartialEq)]
#[rustfmt::skip]
pub enum DatamodelError {
  #[error("Argument \"{}\" is missing.", argument_name)]
  ArgumentNotFound { argument_name: String, span: Span },

  #[error("Function \"{}\" takes {} arguments, but received {}.", function_name, required_count, given_count)]
  ArgumentCountMissmatch { function_name: String, required_count: usize, given_count: usize, span: Span },

  #[error("Argument \"{}\" is missing in attribute \"@{}\".", argument_name, attribute_name)]
  AttributeArgumentNotFound { argument_name: String, attribute_name: String, span: Span },

  #[error("Argument \"{}\" is missing in data source block \"{}\".", argument_name, source_name)]
  SourceArgumentNotFound { argument_name: String, source_name: String, span: Span },

  #[error("Argument \"{}\" is missing in generator block \"{}\".", argument_name, generator_name)]
  GeneratorArgumentNotFound { argument_name: String, generator_name: String, span: Span },

  #[error("Error parsing attribute \"@{}\": {}", attribute_name, message)]
  AttributeValidationError { message: String, attribute_name: String, span: Span },

  #[error("Attribute \"@{}\" is defined twice.", attribute_name)]
  DuplicateAttributeError { attribute_name: String, span: Span },

  #[error("The model with database name \"{}\" could not be defined because another model with this name exists: \"{}\"", model_database_name, existing_model_name)]
  DuplicateModelDatabaseNameError { model_database_name: String, existing_model_name: String, span: Span },

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

  #[error("Function not known: \"{}\".", function_name)]
  FunctionNotKnownError { function_name: String, span: Span },

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
  ParserError { expected: Vec<&'static str>, expected_str: String, span: Span },

  #[error("{}", message)]
  LegacyParserError { message: String, span: Span },

  #[error("{}", message)]
  ConnectorError {message: String, span: Span },

  #[error("{}", message)]
  FunctionalEvaluationError { message: String, span: Span },

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
}

impl DatamodelError {
    pub fn new_literal_parser_error(literal_type: &str, raw_value: &str, span: Span) -> DatamodelError {
        DatamodelError::LiteralParseError {
            literal_type: String::from(literal_type),
            raw_value: String::from(raw_value),
            span,
        }
    }

    pub fn new_argument_not_found_error(argument_name: &str, span: Span) -> DatamodelError {
        DatamodelError::ArgumentNotFound {
            argument_name: String::from(argument_name),
            span,
        }
    }

    pub fn new_argument_count_missmatch_error(
        function_name: &str,
        required_count: usize,
        given_count: usize,
        span: Span,
    ) -> DatamodelError {
        DatamodelError::ArgumentCountMissmatch {
            function_name: String::from(function_name),
            required_count,
            given_count,
            span,
        }
    }

    pub fn new_attribute_argument_not_found_error(
        argument_name: &str,
        attribute_name: &str,
        span: Span,
    ) -> DatamodelError {
        DatamodelError::AttributeArgumentNotFound {
            argument_name: String::from(argument_name),
            attribute_name: String::from(attribute_name),
            span,
        }
    }

    pub fn new_source_argument_not_found_error(argument_name: &str, source_name: &str, span: Span) -> DatamodelError {
        DatamodelError::SourceArgumentNotFound {
            argument_name: String::from(argument_name),
            source_name: String::from(source_name),
            span,
        }
    }

    pub fn new_generator_argument_not_found_error(
        argument_name: &str,
        generator_name: &str,
        span: Span,
    ) -> DatamodelError {
        DatamodelError::GeneratorArgumentNotFound {
            argument_name: String::from(argument_name),
            generator_name: String::from(generator_name),
            span,
        }
    }

    pub fn new_attribute_validation_error(message: &str, attribute_name: &str, span: Span) -> DatamodelError {
        DatamodelError::AttributeValidationError {
            message: String::from(message),
            attribute_name: String::from(attribute_name),
            span,
        }
    }

    pub fn new_duplicate_attribute_error(attribute_name: &str, span: Span) -> DatamodelError {
        DatamodelError::DuplicateAttributeError {
            attribute_name: String::from(attribute_name),
            span,
        }
    }

    pub fn new_reserved_scalar_type_error(type_name: &str, span: Span) -> DatamodelError {
        DatamodelError::ReservedScalarTypeError {
            type_name: String::from(type_name),
            span,
        }
    }

    pub fn new_duplicate_model_database_name_error(
        model_database_name: String,
        existing_model_name: String,
        span: Span,
    ) -> DatamodelError {
        DatamodelError::DuplicateModelDatabaseNameError {
            model_database_name,
            existing_model_name,
            span,
        }
    }

    pub fn new_duplicate_top_error(name: &str, top_type: &str, existing_top_type: &str, span: Span) -> DatamodelError {
        DatamodelError::DuplicateTopError {
            name: String::from(name),
            top_type: String::from(top_type),
            existing_top_type: String::from(existing_top_type),
            span,
        }
    }

    pub fn new_duplicate_config_key_error(conf_block_name: &str, key_name: &str, span: Span) -> DatamodelError {
        DatamodelError::DuplicateConfigKeyError {
            conf_block_name: String::from(conf_block_name),
            key_name: String::from(key_name),
            span,
        }
    }

    pub fn new_duplicate_argument_error(arg_name: &str, span: Span) -> DatamodelError {
        DatamodelError::DuplicateArgumentError {
            arg_name: String::from(arg_name),
            span,
        }
    }

    pub fn new_unused_argument_error(arg_name: &str, span: Span) -> DatamodelError {
        DatamodelError::UnusedArgumentError {
            arg_name: String::from(arg_name),
            span,
        }
    }

    pub fn new_duplicate_default_argument_error(arg_name: &str, span: Span) -> DatamodelError {
        DatamodelError::DuplicateDefaultArgumentError {
            arg_name: String::from(arg_name),
            span,
        }
    }

    pub fn new_duplicate_enum_value_error(enum_name: &str, value_name: &str, span: Span) -> DatamodelError {
        DatamodelError::DuplicateEnumValueError {
            enum_name: String::from(enum_name),
            value_name: String::from(value_name),
            span,
        }
    }

    pub fn new_composite_type_duplicate_field_error(type_name: &str, field_name: &str, span: Span) -> DatamodelError {
        DatamodelError::DuplicateFieldError {
            container_type: "composite type",
            model_name: String::from(type_name),
            field_name: String::from(field_name),
            span,
        }
    }

    pub fn new_duplicate_field_error(model_name: &str, field_name: &str, span: Span) -> DatamodelError {
        DatamodelError::DuplicateFieldError {
            container_type: "model",
            model_name: String::from(model_name),
            field_name: String::from(field_name),
            span,
        }
    }

    pub fn new_scalar_list_fields_are_not_supported(model_name: &str, field_name: &str, span: Span) -> DatamodelError {
        DatamodelError::ScalarListFieldsAreNotSupported {
            model_name: String::from(model_name),
            field_name: String::from(field_name),
            span,
        }
    }

    pub fn new_model_validation_error(message: &str, model_name: &str, span: Span) -> DatamodelError {
        DatamodelError::ModelValidationError {
            message: String::from(message),
            model_name: String::from(model_name),
            span,
        }
    }

    pub fn new_composite_type_validation_error(
        message: String,
        composite_type_name: String,
        span: Span,
    ) -> DatamodelError {
        DatamodelError::CompositeTypeValidationError {
            message,
            composite_type_name,
            span,
        }
    }

    pub fn new_enum_validation_error(message: String, enum_name: String, span: Span) -> DatamodelError {
        DatamodelError::EnumValidationError {
            message,
            enum_name,
            span,
        }
    }

    pub fn new_composite_type_field_validation_error(
        message: &str,
        composite_type_name: &str,
        field: &str,
        span: Span,
    ) -> DatamodelError {
        DatamodelError::FieldValidationError {
            message: message.to_owned(),
            container_name: composite_type_name.to_owned(),
            container_type: "composite type",
            field: field.to_owned(),
            span,
        }
    }

    pub fn new_field_validation_error(message: &str, model: &str, field: &str, span: Span) -> DatamodelError {
        DatamodelError::FieldValidationError {
            message: message.to_owned(),
            container_name: model.to_owned(),
            container_type: "model",
            field: field.to_owned(),
            span,
        }
    }

    pub fn new_source_validation_error(message: &str, source: &str, span: Span) -> DatamodelError {
        DatamodelError::SourceValidationError {
            message: message.to_owned(),
            datasource: source.to_owned(),
            span,
        }
    }

    pub fn new_validation_error(message: String, span: Span) -> DatamodelError {
        DatamodelError::ValidationError { message, span }
    }

    pub fn new_legacy_parser_error(message: &str, span: Span) -> DatamodelError {
        DatamodelError::LegacyParserError {
            message: String::from(message),
            span,
        }
    }

    pub fn new_connector_error(message: &str, span: Span) -> DatamodelError {
        DatamodelError::ConnectorError {
            message: String::from(message),
            span,
        }
    }

    pub fn new_parser_error(expected: &[&'static str], span: Span) -> DatamodelError {
        DatamodelError::ParserError {
            expected: expected.to_owned(),
            expected_str: expected.join(", "),
            span,
        }
    }

    pub fn new_functional_evaluation_error(message: &str, span: Span) -> DatamodelError {
        DatamodelError::FunctionalEvaluationError {
            message: String::from(message),
            span,
        }
    }

    pub fn new_environment_functional_evaluation_error(var_name: String, span: Span) -> DatamodelError {
        DatamodelError::EnvironmentFunctionalEvaluationError { var_name, span }
    }

    pub fn new_type_not_found_error(type_name: &str, span: Span) -> DatamodelError {
        DatamodelError::TypeNotFoundError {
            type_name: String::from(type_name),
            span,
        }
    }

    pub fn new_scalar_type_not_found_error(type_name: &str, span: Span) -> DatamodelError {
        DatamodelError::ScalarTypeNotFoundError {
            type_name: String::from(type_name),
            span,
        }
    }

    pub fn new_attribute_not_known_error(attribute_name: &str, span: Span) -> DatamodelError {
        DatamodelError::AttributeNotKnownError {
            attribute_name: String::from(attribute_name),
            span,
        }
    }

    pub fn new_function_not_known_error(function_name: &str, span: Span) -> DatamodelError {
        DatamodelError::FunctionNotKnownError {
            function_name: String::from(function_name),
            span,
        }
    }

    pub fn new_datasource_provider_not_known_error(provider: &str, span: Span) -> DatamodelError {
        DatamodelError::DatasourceProviderNotKnownError {
            provider: String::from(provider),
            span,
        }
    }

    pub fn new_shadow_database_is_same_as_main_url_error(source_name: String, span: Span) -> DatamodelError {
        DatamodelError::ShadowDatabaseUrlIsSameAsMainUrl { source_name, span }
    }

    pub fn new_preview_feature_not_known_error(
        preview_feature: &str,
        expected_preview_features: String,
        span: Span,
    ) -> DatamodelError {
        DatamodelError::PreviewFeatureNotKnownError {
            preview_feature: String::from(preview_feature),
            expected_preview_features,
            span,
        }
    }

    pub fn new_value_parser_error(expected_type: &str, parser_error: &str, raw: &str, span: Span) -> DatamodelError {
        DatamodelError::ValueParserError {
            expected_type: String::from(expected_type),
            parser_error: String::from(parser_error),
            raw: String::from(raw),
            span,
        }
    }

    pub fn new_type_mismatch_error(expected_type: &str, received_type: &str, raw: &str, span: Span) -> DatamodelError {
        DatamodelError::TypeMismatchError {
            expected_type: String::from(expected_type),
            received_type: String::from(received_type),
            raw: String::from(raw),
            span,
        }
    }

    pub fn span(&self) -> Span {
        match self {
            DatamodelError::ArgumentNotFound { span, .. } => *span,
            DatamodelError::AttributeArgumentNotFound { span, .. } => *span,
            DatamodelError::ArgumentCountMissmatch { span, .. } => *span,
            DatamodelError::SourceArgumentNotFound { span, .. } => *span,
            DatamodelError::GeneratorArgumentNotFound { span, .. } => *span,
            DatamodelError::AttributeValidationError { span, .. } => *span,
            DatamodelError::AttributeNotKnownError { span, .. } => *span,
            DatamodelError::ReservedScalarTypeError { span, .. } => *span,
            DatamodelError::FunctionNotKnownError { span, .. } => *span,
            DatamodelError::DatasourceProviderNotKnownError { span, .. } => *span,
            DatamodelError::LiteralParseError { span, .. } => *span,
            DatamodelError::TypeNotFoundError { span, .. } => *span,
            DatamodelError::ScalarTypeNotFoundError { span, .. } => *span,
            DatamodelError::ParserError { span, .. } => *span,
            DatamodelError::FunctionalEvaluationError { span, .. } => *span,
            DatamodelError::EnvironmentFunctionalEvaluationError { span, .. } => *span,
            DatamodelError::TypeMismatchError { span, .. } => *span,
            DatamodelError::ValueParserError { span, .. } => *span,
            DatamodelError::ValidationError { span, .. } => *span,
            DatamodelError::LegacyParserError { span, .. } => *span,
            DatamodelError::ModelValidationError { span, .. } => *span,
            DatamodelError::DuplicateAttributeError { span, .. } => *span,
            DatamodelError::DuplicateConfigKeyError { span, .. } => *span,
            DatamodelError::DuplicateTopError { span, .. } => *span,
            DatamodelError::DuplicateFieldError { span, .. } => *span,
            DatamodelError::DuplicateEnumValueError { span, .. } => *span,
            DatamodelError::DuplicateArgumentError { span, .. } => *span,
            DatamodelError::DuplicateModelDatabaseNameError { span, .. } => *span,
            DatamodelError::DuplicateDefaultArgumentError { span, .. } => *span,
            DatamodelError::UnusedArgumentError { span, .. } => *span,
            DatamodelError::ScalarListFieldsAreNotSupported { span, .. } => *span,
            DatamodelError::FieldValidationError { span, .. } => *span,
            DatamodelError::SourceValidationError { span, .. } => *span,
            DatamodelError::EnumValidationError { span, .. } => *span,
            DatamodelError::ConnectorError { span, .. } => *span,
            DatamodelError::PreviewFeatureNotKnownError { span, .. } => *span,
            DatamodelError::ShadowDatabaseUrlIsSameAsMainUrl { span, .. } => *span,
            DatamodelError::CompositeTypeValidationError { span, .. } => *span,
        }
    }
    pub fn description(&self) -> String {
        format!("{}", self)
    }

    pub fn pretty_print(&self, f: &mut dyn std::io::Write, file_name: &str, text: &str) -> std::io::Result<()> {
        pretty_print(f, file_name, text, self.span(), self.description().as_str())
    }
}

impl From<schema_ast::parser::ParserError> for DatamodelError {
    fn from(err: schema_ast::parser::ParserError) -> Self {
        match err {
            schema_ast::parser::ParserError::ParserError(message, span) => {
                DatamodelError::new_parser_error(&message, span)
            }
            schema_ast::parser::ParserError::ValidationError(message, span) => {
                DatamodelError::new_validation_error(message, span)
            }
            schema_ast::parser::ParserError::LegacyParserError(message, span) => {
                DatamodelError::new_legacy_parser_error(message, span)
            }
            schema_ast::parser::ParserError::EnumValidationError(message, enum_name, span) => {
                DatamodelError::new_enum_validation_error(message, enum_name, span)
            }
            schema_ast::parser::ParserError::ModelValidationError(message, model_name, span) => {
                DatamodelError::new_model_validation_error(&message, &model_name, span)
            }
        }
    }
}
