use super::*;
use fmt::Debug;
use once_cell::sync::OnceCell;
use prisma_models::{dml, prelude::ParentContainer};
use std::{boxed::Box, fmt, sync::Arc};

#[derive(PartialEq)]
pub struct InputObjectType {
    pub identifier: Identifier,
    pub constraints: InputObjectTypeConstraints,
    pub fields: OnceCell<Vec<InputFieldRef>>,
    pub tag: Option<ObjectTag>,
}

/// Object tags help differentiating objects during parsing / raw input data processing,
/// especially if complex object unions are present.
#[derive(Debug, PartialEq, Clone)]
pub enum ObjectTag {
    CompositeEnvelope,
    RelationEnvelope,
    // Holds the type against which a field can be compared
    FieldRefType(InputType),
    WhereInputType(ParentContainer),
    NestedToOneUpdateEnvelope,
}

#[derive(Debug, Default, PartialEq)]
pub struct InputObjectTypeConstraints {
    /// The maximum number of fields that can be provided.
    pub min_num_fields: Option<usize>,

    /// The minimum number of fields that must be provided.
    pub max_num_fields: Option<usize>,

    /// The fields against which the constraints should be applied.
    /// If `None`, constraints should be applied on _all_ fields on the input object type.
    pub fields: Option<Vec<String>>,
}

impl Debug for InputObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InputObjectType")
            .field("identifier", &self.identifier)
            .field("constraints", &self.constraints)
            .field("fields", &"#Input Fields Cell#")
            .finish()
    }
}

impl InputObjectType {
    pub fn get_fields(&self) -> &Vec<InputFieldRef> {
        self.fields.get().unwrap()
    }

    pub fn set_fields(&self, fields: Vec<InputField>) {
        self.fields
            .set(fields.into_iter().map(Arc::new).collect())
            .unwrap_or_else(|_| panic!("Fields of {:?} are already set", self.identifier));
    }

    /// True if fields are empty, false otherwise.
    pub fn is_empty(&self) -> bool {
        self.get_fields().is_empty()
    }

    pub fn find_field<T>(&self, name: T) -> Option<InputFieldRef>
    where
        T: Into<String>,
    {
        let name = name.into();
        self.get_fields().iter().find(|f| f.name == name).cloned()
    }

    /// Require exactly one field of the possible ones to be in the input.
    pub fn require_exactly_one_field(&mut self) {
        self.set_max_fields(1);
        self.set_min_fields(1);
    }

    /// Require at least one field of the possible ones to be in the input.
    pub fn require_at_least_one_field(&mut self) {
        self.set_min_fields(1);
    }

    /// Require at most one field of the possible ones to be in the input.
    pub fn require_at_most_one_field(&mut self) {
        self.set_max_fields(1);
        self.set_min_fields(0);
    }

    /// Require a maximum of `max` fields to be present in the input.
    pub fn set_max_fields(&mut self, max: usize) {
        self.constraints.max_num_fields = Some(max);
    }

    /// Require a minimum of `min` fields to be present in the input.
    pub fn set_min_fields(&mut self, min: usize) {
        self.constraints.min_num_fields = Some(min);
    }

    pub fn apply_constraints_on_fields(&mut self, fields: Vec<String>) {
        self.constraints.fields = Some(fields);
    }

    pub fn set_tag(&mut self, tag: ObjectTag) {
        self.tag = Some(tag);
    }
}

#[derive(Debug, PartialEq)]
pub struct InputField {
    pub name: String,
    pub default_value: Option<dml::DefaultValue>,
    pub deprecation: Option<Deprecation>,

    /// Possible field types, represented as a union of input types, but only one can be provided at any time.
    pub field_types: Vec<InputType>,

    /// Indicates if the presence of the field on the higher input objects
    /// is required, but doesn't state whether or not the input can be null.
    pub is_required: bool,
}

impl InputField {
    /// Sets the field as optional (not required to be present on the input).
    pub fn optional(mut self) -> Self {
        self.is_required = false;
        self
    }

    /// Sets the field as optional (not required to be present on the input).
    pub fn required(mut self) -> Self {
        self.is_required = true;
        self
    }

    /// Sets the field as optional if the condition is true.
    pub fn optional_if(self, condition: bool) -> Self {
        if condition {
            self.optional()
        } else {
            self
        }
    }

    /// Sets the field as nullable (accepting null inputs).
    pub fn nullable(self) -> Self {
        self.add_type(InputType::null())
    }

    /// Sets the field as nullable if the condition is true.
    pub fn nullable_if(self, condition: bool) -> Self {
        if condition {
            self.nullable()
        } else {
            self
        }
    }

    /// Adds possible input type to this input field's type union.
    pub fn add_type(mut self, typ: InputType) -> Self {
        self.field_types.push(typ);
        self
    }

    pub fn deprecate<T, S>(mut self, reason: T, since_version: S, planned_removal_version: Option<S>) -> Self
    where
        T: Into<String>,
        S: Into<String>,
    {
        self.deprecation = Some(Deprecation {
            reason: reason.into(),
            since_version: since_version.into(),
            planned_removal_version: planned_removal_version.map(Into::into),
        });

        self
    }
}

#[derive(Clone)]
pub enum InputType {
    Scalar(ScalarType),
    Enum(EnumTypeWeakRef),
    List(Box<InputType>),
    Object(InputObjectTypeWeakRef),
}

impl PartialEq for InputType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (InputType::Scalar(st), InputType::Scalar(ost)) => st.eq(ost),
            (InputType::Enum(_), InputType::Enum(_)) => true,
            (InputType::List(lt), InputType::List(olt)) => lt.eq(olt),
            (InputType::Object(obj), InputType::Object(oobj)) => {
                obj.into_arc().identifier == oobj.into_arc().identifier
            }
            _ => false,
        }
    }
}

impl Debug for InputType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputType::Object(obj) => write!(f, "Object({})", obj.into_arc().identifier.name()),
            InputType::Scalar(s) => write!(f, "{s:?}"),
            InputType::Enum(e) => write!(f, "{e:?}"),
            InputType::List(l) => write!(f, "{l:?}"),
        }
    }
}

impl InputType {
    pub fn list(containing: InputType) -> InputType {
        InputType::List(Box::new(containing))
    }

    pub fn object(containing: InputObjectTypeWeakRef) -> InputType {
        InputType::Object(containing)
    }

    pub fn string() -> InputType {
        InputType::Scalar(ScalarType::String)
    }

    pub fn int() -> InputType {
        InputType::Scalar(ScalarType::Int)
    }

    pub fn bigint() -> InputType {
        InputType::Scalar(ScalarType::BigInt)
    }

    pub fn float() -> InputType {
        InputType::Scalar(ScalarType::Float)
    }

    pub fn decimal() -> InputType {
        InputType::Scalar(ScalarType::Decimal)
    }

    pub fn boolean() -> InputType {
        InputType::Scalar(ScalarType::Boolean)
    }

    pub fn date_time() -> InputType {
        InputType::Scalar(ScalarType::DateTime)
    }

    pub fn json() -> InputType {
        InputType::Scalar(ScalarType::Json)
    }

    pub fn json_list() -> InputType {
        InputType::Scalar(ScalarType::JsonList)
    }

    pub fn uuid() -> InputType {
        InputType::Scalar(ScalarType::UUID)
    }

    pub fn xml() -> InputType {
        InputType::Scalar(ScalarType::Xml)
    }

    pub fn bytes() -> InputType {
        InputType::Scalar(ScalarType::Bytes)
    }

    pub fn null() -> InputType {
        InputType::Scalar(ScalarType::Null)
    }

    pub fn enum_type(containing: EnumTypeWeakRef) -> InputType {
        InputType::Enum(containing)
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Scalar(_) => false,
            Self::Enum(_) => false,
            Self::List(inner) => inner.is_empty(),
            Self::Object(weak) => weak.into_arc().is_empty(),
        }
    }

    pub fn is_json(&self) -> bool {
        matches!(
            self,
            Self::Scalar(ScalarType::Json) | Self::Scalar(ScalarType::JsonList)
        )
    }

    pub fn as_object(&self) -> Option<InputObjectTypeStrongRef> {
        if let Self::Object(v) = self {
            Some(v.into_arc())
        } else {
            None
        }
    }

    pub fn as_list(&self) -> Option<&InputType> {
        if let Self::List(list) = self {
            Some(list)
        } else {
            None
        }
    }
}

impl From<InputType> for Vec<InputType> {
    fn from(r#type: InputType) -> Self {
        vec![r#type]
    }
}
