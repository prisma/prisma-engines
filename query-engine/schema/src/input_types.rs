use super::*;
use fmt::Debug;
use query_structure::{DefaultKind, prelude::ParentContainer};
use std::{borrow::Cow, boxed::Box, fmt, sync::LazyLock};

type InputObjectFields<'a> =
    Option<Arc<LazyLock<Vec<InputField<'a>>, Box<dyn FnOnce() -> Vec<InputField<'a>> + Send + Sync + 'a>>>>;

#[derive(Clone)]
pub struct InputObjectType<'a> {
    pub identifier: Identifier,
    pub constraints: InputObjectTypeConstraints<'a>,
    pub(crate) fields: InputObjectFields<'a>,
    pub(crate) tag: Option<ObjectTag<'a>>,
    pub(crate) container: Option<ParentContainer>,
}

impl PartialEq for InputObjectType<'_> {
    #[allow(unconditional_recursion)]
    fn eq(&self, other: &Self) -> bool {
        self.identifier.eq(&other.identifier)
    }
}

/// Object tags help differentiating objects during parsing / raw input data processing,
/// especially if complex object unions are present.
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectTag<'a> {
    CompositeEnvelope,
    RelationEnvelope,
    // Holds the type against which a field can be compared
    FieldRefType(Box<InputType<'a>>),
    WhereInputType(ParentContainer),
    NestedToOneUpdateEnvelope,
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct InputObjectTypeConstraints<'a> {
    /// The maximum number of fields that can be provided.
    pub min_num_fields: Option<usize>,

    /// The minimum number of fields that must be provided.
    pub max_num_fields: Option<usize>,

    /// The fields against which the constraints should be applied.
    /// If `None`, constraints should be applied on _all_ fields on the input object type.
    pub fields: Option<Vec<Cow<'a, str>>>,
}

impl Debug for InputObjectType<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InputObjectType")
            .field("identifier", &self.identifier)
            .field("constraints", &self.constraints)
            .field("fields", &"#Input Fields Cell#")
            .field("container", &self.container)
            .finish()
    }
}

impl<'a> InputObjectType<'a> {
    pub fn get_fields(&self) -> &[InputField<'a>] {
        self.fields.as_ref().map(|f| -> &[_] { f }).unwrap_or(&[])
    }

    pub(crate) fn set_fields(&mut self, f: impl FnOnce() -> Vec<InputField<'a>> + Send + Sync + 'a) {
        self.fields = Some(Arc::new(LazyLock::new(Box::new(f))));
    }

    pub fn tag(&self) -> Option<&ObjectTag<'a>> {
        self.tag.as_ref()
    }

    pub fn container(&self) -> Option<&ParentContainer> {
        self.container.as_ref()
    }

    pub fn set_container(&mut self, container: impl Into<ParentContainer>) {
        self.container = Some(container.into());
    }

    pub fn find_field(&self, name: &str) -> Option<&InputField<'a>> {
        self.get_fields().iter().find(|f| f.name == name)
    }

    /// Require exactly one field of the possible ones to be in the input.
    pub(crate) fn require_exactly_one_field(&mut self) {
        self.set_max_fields(1);
        self.set_min_fields(1);
    }

    /// Require at least one field of the possible ones to be in the input.
    pub(crate) fn require_at_least_one_field(&mut self) {
        self.set_min_fields(1);
    }

    /// Require at most one field of the possible ones to be in the input.
    pub(crate) fn require_at_most_one_field(&mut self) {
        self.set_max_fields(1);
        self.set_min_fields(0);
    }

    /// Require a maximum of `max` fields to be present in the input.
    pub(crate) fn set_max_fields(&mut self, max: usize) {
        self.constraints.max_num_fields = Some(max);
    }

    /// Require a minimum of `min` fields to be present in the input.
    pub(crate) fn set_min_fields(&mut self, min: usize) {
        self.constraints.min_num_fields = Some(min);
    }

    pub(crate) fn apply_constraints_on_fields(&mut self, fields: Vec<Cow<'a, str>>) {
        self.constraints.fields = Some(fields);
    }

    pub(crate) fn set_tag(&mut self, tag: ObjectTag<'a>) {
        self.tag = Some(tag);
    }
}

#[derive(Debug, Clone)]
pub struct InputField<'a> {
    pub name: Cow<'a, str>,
    pub default_value: Option<DefaultKind>,

    field_types: Vec<InputType<'a>>,
    is_required: bool,
}

impl<'a> InputField<'a> {
    pub(crate) fn new(
        name: Cow<'a, str>,
        field_types: Vec<InputType<'a>>,
        default_value: Option<DefaultKind>,
        is_required: bool,
    ) -> InputField<'a> {
        InputField {
            name,
            default_value,
            field_types,
            is_required,
        }
    }

    pub fn field_types(&self) -> &[InputType<'a>] {
        &self.field_types
    }

    /// Indicates if the presence of the field on the higher input objects
    /// is required, but doesn't state whether or not the input can be null.
    pub fn is_required(&self) -> bool {
        self.is_required
    }

    /// Sets the field as optional (not required to be present on the input).
    pub(crate) fn optional(mut self) -> Self {
        self.is_required = false;
        self
    }

    /// Sets the field as optional (not required to be present on the input).
    pub(crate) fn required(mut self) -> Self {
        self.is_required = true;
        self
    }

    /// Sets the field as optional if the condition is true.
    pub(crate) fn optional_if(self, condition: bool) -> Self {
        if condition { self.optional() } else { self }
    }

    /// Sets the field as nullable (accepting null inputs).
    pub(crate) fn nullable(mut self) -> Self {
        // self.field_types = Box::new(|| {
        //     let f = &self.field_types;
        //     let mut fields = f();
        //     fields.push(InputType::null());
        //     fields
        // });
        self.field_types.push(InputType::null());
        self
    }

    /// Sets the field as nullable if the condition is true.
    pub(crate) fn nullable_if(self, condition: bool) -> Self {
        if condition { self.nullable() } else { self }
    }
}

#[derive(Clone)]
pub enum InputType<'a> {
    Scalar(ScalarType),
    Enum(EnumType),
    List(Box<InputType<'a>>),
    Object(InputObjectType<'a>),
}

impl PartialEq for InputType<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (InputType::Scalar(st), InputType::Scalar(ost)) => st.eq(ost),
            (InputType::Enum(_), InputType::Enum(_)) => true,
            (InputType::List(lt), InputType::List(olt)) => lt.eq(olt),
            (InputType::Object(obj), InputType::Object(oobj)) => obj == oobj,
            _ => false,
        }
    }
}

impl Debug for InputType<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Object(obj) => write!(f, "Object({obj:?})"),
            Self::Scalar(s) => write!(f, "{s:?}"),
            Self::Enum(e) => write!(f, "{e:?}"),
            Self::List(l) => write!(f, "{l:?}"),
        }
    }
}

impl<'a> InputType<'a> {
    pub(crate) fn list(containing: InputType<'a>) -> InputType<'a> {
        InputType::List(Box::new(containing))
    }

    pub(crate) fn object(containing: InputObjectType<'a>) -> InputType<'a> {
        InputType::Object(containing)
    }

    pub(crate) fn string() -> InputType<'a> {
        InputType::Scalar(ScalarType::String)
    }

    pub(crate) fn int() -> InputType<'a> {
        InputType::Scalar(ScalarType::Int)
    }

    pub(crate) fn bigint() -> InputType<'a> {
        InputType::Scalar(ScalarType::BigInt)
    }

    pub(crate) fn float() -> InputType<'a> {
        InputType::Scalar(ScalarType::Float)
    }

    pub(crate) fn decimal() -> InputType<'a> {
        InputType::Scalar(ScalarType::Decimal)
    }

    pub(crate) fn boolean() -> InputType<'a> {
        InputType::Scalar(ScalarType::Boolean)
    }

    pub(crate) fn date_time() -> InputType<'a> {
        InputType::Scalar(ScalarType::DateTime)
    }

    pub(crate) fn json() -> InputType<'a> {
        InputType::Scalar(ScalarType::Json)
    }

    pub(crate) fn json_list() -> InputType<'a> {
        InputType::Scalar(ScalarType::JsonList)
    }

    pub(crate) fn uuid() -> InputType<'a> {
        InputType::Scalar(ScalarType::UUID)
    }

    pub(crate) fn bytes() -> InputType<'a> {
        InputType::Scalar(ScalarType::Bytes)
    }

    pub(crate) fn null() -> InputType<'a> {
        InputType::Scalar(ScalarType::Null)
    }

    pub(crate) fn enum_type(containing: EnumType) -> InputType<'a> {
        InputType::Enum(containing)
    }

    pub fn is_json(&self) -> bool {
        matches!(
            self,
            Self::Scalar(ScalarType::Json) | Self::Scalar(ScalarType::JsonList)
        )
    }

    pub fn as_object(&self) -> Option<&InputObjectType<'a>> {
        if let Self::Object(v) = self { Some(v) } else { None }
    }

    pub fn as_list(&self) -> Option<&InputType<'a>> {
        if let Self::List(list) = self { Some(list) } else { None }
    }

    pub fn into_object(self) -> Option<InputObjectType<'a>> {
        match self {
            InputType::Object(obj) => Some(obj),
            _ => None,
        }
    }
}
