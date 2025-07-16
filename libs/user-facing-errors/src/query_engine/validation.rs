use crate::KnownError;
use itertools::Itertools;
use serde::Serialize;
use serde_json::json;
use std::{borrow::Cow, error, fmt};

/// A validation error is a Serializable object that contains the path where the validation error
/// of a certain `kind` ocurred, and an optional and arbitrary piece of `meta`-information.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationError {
    kind: ValidationErrorKind,
    #[serde(skip)]
    message: String,
    #[serde(flatten)]
    meta: Option<serde_json::Value>,
}

impl ValidationError {
    pub fn kind(&self) -> &ValidationErrorKind {
        &self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

#[derive(Debug, Serialize)]
pub enum ValidationErrorKind {
    /// See [`ValidationError::unexpected_runtime_error`].
    UnexpectedRuntimeError,
    /// See [`ValidationError::empty_selection`]
    EmptySelection,
    ///See [`ValidationError::invalid_argument_type`]
    InvalidArgumentType,
    ///See [`ValidationError::invalid_argument_value`]
    InvalidArgumentValue,
    /// See [`ValidationError::some_fields_missing`]
    SomeFieldsMissing,
    /// See [`ValidationError::too_many_fields_given`]
    TooManyFieldsGiven,
    /// See [`ValidationError::selection_set_on_scalar`]
    SelectionSetOnScalar,
    /// See [`ValidationError::required_argument_missing`]
    RequiredArgumentMissing,
    /// See [`ValidationError::union`]
    Union,
    /// See [`ValidationError::unknown_argument`]
    UnknownArgument,
    /// See [`ValidationError::unknown_input_field`]
    UnknownInputField,
    /// See [`ValidationError::unknown_selection_field`]
    UnknownSelectionField,
    /// See [`ValidationError::value_too_large`]
    ValueTooLarge,
}

impl ValidationErrorKind {
    /// Returns the appropriate code code for the different validation errors.
    ///
    /// TODO: Ideally each all validation errors should have the same error code (P2009), or distinct
    /// type each of them should have an individual error code. For the time being, we keep the
    /// semantics documented in the [error reference][r] as users might be relying on the error
    /// codes when subscribing to error events. Otherwise, we could be introducing a breaking change.
    ///
    /// [r]: https://www.prisma.io/docs/reference/api-reference/error-reference
    pub fn code(&self) -> &'static str {
        match self {
            ValidationErrorKind::RequiredArgumentMissing => "P2012",
            _ => "P2009",
        }
    }
}

impl From<ValidationError> for crate::KnownError {
    fn from(err: ValidationError) -> Self {
        KnownError {
            message: err.message.clone(),
            meta: serde_json::to_value(&err).expect("Failed to render validation error to JSON"),
            error_code: Cow::from(err.kind.code()),
        }
    }
}

impl From<ValidationError> for crate::Error {
    fn from(err: ValidationError) -> Self {
        KnownError::from(err).into()
    }
}

impl ValidationError {
    /// Creates an [`ValidationErrorKind::UnexpectedRuntimeError`] kind of error when something unexpected
    /// happen at runtime after a query was properly validated by the parser against the schema.
    pub fn unexpected_runtime_error(message: String) -> Self {
        ValidationError {
            kind: ValidationErrorKind::UnexpectedRuntimeError,
            message,
            meta: None,
        }
    }

    /// Creates an [`ValidationErrorKind::EmptySelection`] kind of error, which happens when the
    /// selection of fields is empty for a query.
    ///
    /// Example json query:
    ///
    /// {
    ///     "action": "findMany",
    ///     "modelName": "User",
    ///     "query": {
    ///         "selection": {}
    ///     }
    /// }
    pub fn empty_selection(selection_path: Vec<&str>, output_type_description: OutputTypeDescription) -> Self {
        let message = String::from("Expected a minimum of 1 field to be present, got 0");
        ValidationError {
            kind: ValidationErrorKind::EmptySelection,
            message,
            meta: Some(json!({ "outputType": output_type_description, "selectionPath": selection_path })),
        }
    }

    /// Creates an [`ValidationErrorKind::InvalidArgumentType`] kind of error, which happens when the
    /// argument is of a type that is incompatible with its definition.
    ///
    /// Say the schema type for user.id is `Int`
    ///
    /// The example json query will fail, as it's trying to pass a string instead.
    ///
    /// {
    ///     "action": "findMany",
    ///     "modelName": "User",
    ///     "query": {
    ///         "arguments": {
    ///             "where": {
    ///                 "id": "a22b8732-be32-4a30-9b38-78843aaa48f8"
    ///             }
    ///         },
    ///         "selection": {
    ///             "$scalars": true
    ///         }
    ///     }
    /// }
    pub fn invalid_argument_type(
        selection_path: Vec<&str>,
        argument_path: Vec<&str>,
        argument_description: ArgumentDescription<'_>,
        inferred_argument_type: String,
    ) -> Self {
        let message = format!(
            "Invalid argument type. `{}` should be of any of the following types: `{}`",
            argument_description.name,
            argument_description.type_names.join(", ")
        );
        ValidationError {
            kind: ValidationErrorKind::InvalidArgumentType,
            message,
            meta: Some(
                json!({"argumentPath": argument_path, "argument": argument_description, "selectionPath": selection_path, "inferredType": inferred_argument_type }),
            ),
        }
    }

    /// Creates an [`ValidationErrorKind::InvalidArgumentValue`] kind of error, which happens when the
    /// argument is of the correct type, but its value is invalid, said a negative number on a type
    /// that is integer but which values should be non-negative. Or a uuid which type is correctly
    /// a string, but its format is not the appropriate.
    ///
    /// Say the schema type for user.id is `Int`
    ///
    /// The example json query will fail, as it's trying to pass a string instead.
    ///
    /// {
    ///     "action": "findMany",
    ///     "modelName": "User",
    ///     "query": {
    ///         "arguments": {
    ///             "where": {
    ///                 "dob": "invalid date"
    ///             }
    ///         },
    ///         "selection": {
    ///             "$scalars": true
    ///         }
    ///     }
    /// }
    pub fn invalid_argument_value(
        selection_path: Vec<&str>,
        argument_path: Vec<&str>,
        value: String,
        expected_argument_type: &str,
        underlying_err: Option<Box<dyn error::Error>>,
    ) -> Self {
        let argument_name = argument_path.last().expect("Argument path cannot not be empty");
        let (message, meta) = if let Some(err) = underlying_err {
            let err_msg = err.to_string();
            let message = format!(
                "Invalid argument agument value. `{}` is not a valid `{}`. Underlying error: {}",
                value, expected_argument_type, &err_msg
            );
            let argument = ArgumentDescription::new(*argument_name, vec![Cow::Borrowed(expected_argument_type)]);
            let meta = json!({"argumentPath": argument_path, "argument": argument, "selectionPath": selection_path, "underlyingError": &err_msg});
            (message, Some(meta))
        } else {
            let message = format!(
                "Invalid argument agument value. `{}` is not a valid `{}`",
                value, &expected_argument_type
            );
            let argument = ArgumentDescription::new(*argument_name, vec![Cow::Borrowed(expected_argument_type)]);
            let meta = json!({"argumentPath": argument_path, "argument": argument, "selectionPath": selection_path, "underlyingError": serde_json::Value::Null});
            (message, Some(meta))
        };
        ValidationError {
            kind: ValidationErrorKind::InvalidArgumentValue,
            message,
            meta,
        }
    }

    /// Creates an [`ValidationErrorKind::SomeFieldsMissing`] kind of error, which happens when
    /// there are some fields missing from a query
    pub fn some_fields_missing(
        selection_path: Vec<&str>,
        argument_path: Vec<&str>,
        min_field_count: Option<usize>,
        max_field_count: Option<usize>,
        required_fields: Option<Vec<Cow<'_, str>>>,
        provided_field_count: usize,
        input_type_description: &InputTypeDescription,
    ) -> Self {
        let constraints =
            InputTypeConstraints::new(min_field_count, max_field_count, required_fields, provided_field_count);
        let message = format!("Some fields are missing: {constraints}");
        ValidationError {
            kind: ValidationErrorKind::SomeFieldsMissing,
            message,
            meta: Some(
                json!({ "inputType": input_type_description, "argumentPath": argument_path, "selectionPath": selection_path, "constraints": constraints }),
            ),
        }
    }

    /// Creates an [`ValidationErrorKind::SomeFieldsMissing`] kind of error, which happens when
    /// there are more fields given than the ones a type accept
    pub fn too_many_fields_given(
        selection_path: Vec<&str>,
        argument_path: Vec<&str>,
        min_field_count: Option<usize>,
        max_field_count: Option<usize>,
        required_fields: Option<Vec<Cow<'_, str>>>,
        provided_field_count: usize,
        input_type_description: &InputTypeDescription,
    ) -> Self {
        let constraints =
            InputTypeConstraints::new(min_field_count, max_field_count, required_fields, provided_field_count);
        let message = format!("Too many fields given: {constraints}");
        ValidationError {
            kind: ValidationErrorKind::TooManyFieldsGiven,
            message,
            meta: Some(
                json!({ "inputType": input_type_description, "argumentPath": argument_path,  "selectionPath": selection_path, "constraints": constraints }),
            ),
        }
    }

    /// Creates an [`ValidationErrorKind::RequiredArgumentMissing`] kind of error, which happens
    /// when there is a missing argument for a field missing, like the `where` field below.
    ///
    /// Example json query:
    ///
    /// {
    ///     "action": "findMany",
    ///     "modelName": "User",
    ///     "query": {
    ///         "selection": {}
    ///     }
    /// }
    ///
    pub fn required_argument_missing(
        selection_path: Vec<&str>,
        argument_path: Vec<&str>,
        input_type_descriptions: &[InputTypeDescription],
    ) -> Self {
        let message = format!("`{}`: A value is required but not set", argument_path.join("."));
        ValidationError {
            kind: ValidationErrorKind::RequiredArgumentMissing,
            message,
            meta: Some(
                json!({ "inputTypes": input_type_descriptions, "argumentPath": argument_path,  "selectionPath": selection_path }),
            ),
        }
    }

    /// Creates an [`ValidationErrorKind::UnknownArgument`] kind of error, which happens when the
    /// arguments for a query are not congruent with those expressed in the schema
    ///
    /// Example json query:
    ///
    /// {
    ///     "action": "findMany",
    ///     "modelName": "User",
    ///     "query": {
    ///         "arguments": {
    ///             "foo": "123"
    ///         },
    ///         "selection": {
    ///             "$scalars": true
    ///         }
    ///     }
    /// }
    /// Todo: add the `given` type to the meta
    pub fn unknown_argument(
        selection_path: Vec<&str>,
        argument_path: Vec<&str>,
        valid_argument_descriptions: Vec<ArgumentDescription<'_>>,
    ) -> Self {
        let message = String::from("Argument does not exist in enclosing type");
        ValidationError {
            kind: ValidationErrorKind::UnknownArgument,
            message,
            meta: Some(
                json!({"argumentPath": argument_path, "arguments": valid_argument_descriptions, "selectionPath": selection_path}),
            ),
        }
    }

    /// Creates a [`ValidationErrorKind::UnknownInputField`] kind of error, which happens when the
    /// argument value for a query contains a field that does not exist in the schema for the
    /// input type.
    ///
    /// TODO:
    ///   how is this conceptually different from an unknown argument? This used to be a
    ///   FieldNotFoundError (see [old code][c]), but the same FieldNotFoundError was used to
    ///   denote what's now an UnknownSelectionField.
    ///
    /// [c]: https://www.prisma.io/docs/reference/api-reference/error-reference
    ///
    /// Example json query:
    ///
    /// {
    ///     "action": "findMany",
    ///     "modelName": "User",
    ///     "query": {
    ///         "arguments": {
    ///             "where": {
    ///                 "foo": 2
    ///             }
    ///         },
    ///         "selection": {
    ///             "$scalars": true
    ///         }
    ///     }
    /// }
    ///
    pub fn unknown_input_field(
        selection_path: Vec<&str>,
        argument_path: Vec<&str>,
        input_type_description: InputTypeDescription,
    ) -> Self {
        let message = format!(
            "`{}.{}`: Field does not exist in enclosing type.",
            selection_path.join("."),
            argument_path.join("."),
        );
        ValidationError {
            kind: ValidationErrorKind::UnknownInputField,
            message,
            meta: Some(
                json!({ "inputType": input_type_description, "argumentPath": argument_path, "selectionPath": selection_path }),
            ),
        }
    }

    /// Creates an [`ValidationErrorKind::UnknownSelectionField`] kind of error, which happens when
    /// the selection of fields for a query contains a field that does not exist in the schema for the
    /// enclosing type
    ///
    /// Example json query:
    ///
    /// {
    ///     "action": "findMany",
    ///     "modelName": "User",
    ///     "query": {
    ///         "selection": {
    ///             "notAField": true
    ///         }
    ///     }
    // }
    pub fn unknown_selection_field(selection_path: Vec<&str>, output_type_description: OutputTypeDescription) -> Self {
        let message = format!(
            "Field '{}' not found in enclosing type '{}'",
            selection_path.last().expect("Selection path must not be empty"),
            output_type_description.name
        );
        ValidationError {
            kind: ValidationErrorKind::UnknownSelectionField,
            message,
            meta: Some(json!({ "outputType": output_type_description, "selectionPath": selection_path })),
        }
    }

    /// Creates an [`ValidationErrorKind::SelectionSetOnScalar`] kind of error, which happens when there
    /// is a nested selection block on a scalar field
    ///
    /// Example json query:
    ///
    /// {
    ///     "action": "findMany",
    ///     "modelName": "User",
    ///     "query": {
    ///         "selection": {
    ///             "email": {
    ///                 "selection": {
    ///                     "id": true
    ///                 }
    ///             }
    ///         }
    ///     }
    /// }
    pub fn selection_set_on_scalar(field_name: String, selection_path: Vec<&str>) -> Self {
        let message = format!("Cannot select over scalar field '{}'", &field_name);
        ValidationError {
            kind: ValidationErrorKind::SelectionSetOnScalar,
            message,
            meta: Some(json!({ "fieldName": field_name, "selectionPath": selection_path })),
        }
    }

    /// Creates an error that is the union of different validation errors
    pub fn union(errors: Vec<ValidationError>) -> Self {
        let message = format!(
            "Unable to match input value to any allowed input type for the field. Parse errors: [{}]",
            errors.iter().map(|err| format!("{err}")).collect::<Vec<_>>().join(", ")
        );
        ValidationError {
            message,
            kind: ValidationErrorKind::Union,
            meta: Some(json!({ "errors": errors })),
        }
    }

    /// Creates an [`ValidationErrorKind::ValueTooLarge`] kind of error, which happens when the value
    /// for a float or integer coming from the JS client is larger than what can fit in an i64
    /// (2^64 - 1 = 18446744073709550000)
    ///
    /// Example json query
    ///
    ///{
    ///     "action": "findMany",
    ///     "modelName": "User",
    ///     "query": {
    ///         "arguments": {
    ///             "where": {
    ///                 "id": 18446744073709550000 // too large
    ///             }
    ///         },
    ///         "selection": {
    ///             "$scalars": true
    ///         }
    ///     }
    /// }
    ///
    pub fn value_too_large(selection_path: Vec<&str>, argument_path: Vec<&str>, value: String) -> Self {
        let argument_name = argument_path.last().expect("Argument path cannot not be empty");
        let message = format!(
            "Unable to fit float value (or large JS integer serialized in exponent notation) '{value}' into a 64 Bit signed integer for field '{argument_name}'. If you're trying to store large integers, consider using `BigInt`",
        );
        let argument = ArgumentDescription::new(*argument_name, vec![Cow::Borrowed("BigInt")]);
        ValidationError {
            kind: ValidationErrorKind::ValueTooLarge,
            message,
            meta: Some(json!({"argumentPath": argument_path, "argument": argument, "selectionPath": selection_path})),
        }
    }
}

impl std::error::Error for ValidationError {}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputTypeDescription {
    name: String,
    fields: Vec<OutputTypeDescriptionField>,
}

impl OutputTypeDescription {
    pub fn new(name: String, fields: Vec<OutputTypeDescriptionField>) -> Self {
        OutputTypeDescription { name, fields }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputTypeDescriptionField {
    name: String,
    type_name: String,
    is_relation: bool,
}

impl OutputTypeDescriptionField {
    pub fn new(name: String, type_name: String, is_relation: bool) -> Self {
        Self {
            name,
            type_name,
            is_relation,
        }
    }
}
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum InputTypeDescription {
    Object {
        name: String,
        fields: Vec<InputTypeDescriptionField>,
    },
    Scalar {
        name: String,
    },
    List {
        element_type: Box<InputTypeDescription>,
    },
    Enum {
        name: String,
    },
}

impl InputTypeDescription {
    pub fn new_object(name: String, fields: Vec<InputTypeDescriptionField>) -> Self {
        Self::Object { name, fields }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InputTypeDescriptionField {
    name: String,
    type_names: Vec<String>,
    required: bool,
}

impl InputTypeDescriptionField {
    pub fn new(name: String, type_names: Vec<String>, required: bool) -> Self {
        Self {
            name,
            type_names,
            required,
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct InputTypeConstraints<'a> {
    #[serde(rename = "minFieldCount")]
    min: Option<usize>,
    #[serde(rename = "maxFieldCount")]
    max: Option<usize>,
    #[serde(rename = "requiredFields")]
    fields: Option<Vec<Cow<'a, str>>>,
    #[serde(skip)]
    got: usize,
}

impl<'a> InputTypeConstraints<'a> {
    fn new(min: Option<usize>, max: Option<usize>, fields: Option<Vec<Cow<'a, str>>>, got: usize) -> Self {
        Self { min, max, fields, got }
    }
}

// Todo: we might not need this, having only the two kind of error types related to cardinality
// TooManyFieldsGiven, SomeFieldsMissing
impl fmt::Display for InputTypeConstraints<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.fields {
            None => match (self.min, self.max) {
                (Some(1), Some(1)) => {
                    write!(f, "Expected exactly one field to be present, got {}.", self.got)
                }
                (Some(min), Some(max)) => write!(
                    f,
                    "Expected a minimum of {} and at most {} fields to be present, got {}.",
                    min, max, self.got
                ),
                (Some(min), None) => write!(
                    f,
                    "Expected a minimum of {} fields to be present, got {}.",
                    min, self.got
                ),
                (None, Some(max)) => write!(f, "Expected at most {} fields to be present, got {}.", max, self.got),
                (None, None) => write!(f, "Expected any selection of fields, got {}.", self.got),
            },
            Some(fields) => match (self.min, self.max) {
                (Some(1), Some(1)) => {
                    write!(
                        f,
                        "Expected exactly one field of ({}) to be present, got {}.",
                        fields.iter().join(", "),
                        self.got
                    )
                }
                (Some(min), Some(max)) => write!(
                    f,
                    "Expected a minimum of {} and at most {} fields of ({}) to be present, got {}.",
                    min,
                    max,
                    fields.iter().join(", "),
                    self.got
                ),
                (Some(min), None) => write!(
                    f,
                    "Expected a minimum of {} fields of ({}) to be present, got {}.",
                    min,
                    fields.iter().join(", "),
                    self.got
                ),
                (None, Some(max)) => write!(
                    f,
                    "Expected at most {} fields of ({}) to be present, got {}.",
                    max,
                    fields.iter().join(", "),
                    self.got
                ),
                (None, None) => write!(f, "Expected any selection of fields, got {}.", self.got),
            },
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArgumentDescription<'a> {
    name: Cow<'a, str>,
    type_names: Vec<Cow<'a, str>>,
}

impl<'a> ArgumentDescription<'a> {
    pub fn new(name: impl Into<Cow<'a, str>>, type_names: Vec<Cow<'a, str>>) -> Self {
        Self {
            name: name.into(),
            type_names,
        }
    }
}
