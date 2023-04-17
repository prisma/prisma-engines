use super::*;
use fmt::Debug;
use once_cell::sync::OnceCell;
use prisma_models::{dml, prelude::ParentContainer};
use std::{boxed::Box, fmt};

#[derive(PartialEq)]
pub struct InputObjectType {
    pub identifier: Identifier,
    pub constraints: InputObjectTypeConstraints,
    pub fields: OnceCell<Vec<InputField>>,
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
    pub fn get_fields(&self) -> &Vec<InputField> {
        self.fields.get().unwrap()
    }

    pub fn set_fields(&self, fields: Vec<InputField>) {
        self.fields
            .set(fields)
            .unwrap_or_else(|_| panic!("Fields of {:?} are already set", self.identifier));
    }

    /// True if fields are empty, false otherwise.
    pub fn is_empty(&self) -> bool {
        self.get_fields().is_empty()
    }

    pub fn find_field<T>(&self, name: T) -> Option<&InputField>
    where
        T: Into<String>,
    {
        let name = name.into();
        self.get_fields().iter().find(|f| f.name == name)
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
    pub default_value: Option<dml::DefaultKind>,

    /// Possible field types, represented as a union of input types, but only one can be provided at any time.
    /// Slice expressed as (start, len).
    field_types: Option<(usize, usize)>,

    /// Indicates if the presence of the field on the higher input objects
    /// is required, but doesn't state whether or not the input can be null.
    pub is_required: bool,
}

impl InputField {
    pub fn new(name: String, default_value: Option<dml::DefaultKind>, is_required: bool) -> InputField {
        InputField {
            name,
            default_value,
            field_types: None,
            is_required,
        }
    }

    pub fn field_types<'a>(&self, query_schema: &'a QuerySchema) -> &'a [InputType] {
        let (start, len) = self.field_types.unwrap_or_default();
        &query_schema.db.input_field_types[start..(start + len)]
    }

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
    pub fn nullable(self, db: &mut QuerySchemaDatabase) -> Self {
        self.add_type(InputType::null(), db)
    }

    /// Sets the field as nullable if the condition is true.
    pub fn nullable_if(self, condition: bool, db: &mut QuerySchemaDatabase) -> Self {
        if condition {
            self.nullable(db)
        } else {
            self
        }
    }

    pub fn push_type(&mut self, typ: InputType, db: &mut QuerySchemaDatabase) {
        match &mut self.field_types {
            Some((_start, len)) => {
                *len += 1;
                db.input_field_types.push(typ);
            }
            None => {
                self.field_types = Some((db.input_field_types.len(), 1));
                db.input_field_types.push(typ);
            }
        }
    }

    /// Adds possible input type to this input field's type union.
    pub fn add_type(mut self, typ: InputType, db: &mut QuerySchemaDatabase) -> Self {
        self.push_type(typ, db);
        self
    }
}

#[derive(Clone)]
pub enum InputType {
    Scalar(ScalarType),
    Enum(EnumTypeId),
    List(Box<InputType>),
    Object(InputObjectTypeId),
}

impl PartialEq for InputType {
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

impl Debug for InputType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Object(obj) => write!(f, "Object({obj:?})"),
            Self::Scalar(s) => write!(f, "{s:?}"),
            Self::Enum(e) => write!(f, "{e:?}"),
            Self::List(l) => write!(f, "{l:?}"),
        }
    }
}

impl InputType {
    pub fn list(containing: InputType) -> InputType {
        InputType::List(Box::new(containing))
    }

    pub fn object(containing: InputObjectTypeId) -> InputType {
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

    pub fn enum_type(containing: EnumTypeId) -> InputType {
        InputType::Enum(containing)
    }

    pub fn is_empty(&self, query_schema: &QuerySchemaDatabase) -> bool {
        match self {
            Self::Scalar(_) => false,
            Self::Enum(_) => false,
            Self::List(inner) => inner.is_empty(query_schema),
            Self::Object(id) => query_schema[*id].is_empty(),
        }
    }

    pub fn is_json(&self) -> bool {
        matches!(
            self,
            Self::Scalar(ScalarType::Json) | Self::Scalar(ScalarType::JsonList)
        )
    }

    pub fn as_object<'a>(&self, query_schema: &'a QuerySchemaDatabase) -> Option<&'a InputObjectType> {
        if let Self::Object(v) = self {
            Some(&query_schema[*v])
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

impl IntoIterator for InputType {
    type Item = InputType;
    type IntoIter = std::iter::Once<InputType>;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self)
    }
}
