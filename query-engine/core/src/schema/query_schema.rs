use super::*;
use crate::{ParsedField, QueryGraph, QueryGraphBuilderResult};
use fmt::Debug;
use once_cell::sync::OnceCell;
use prisma_models::{dml, InternalDataModelRef, ModelRef};
use std::{
    borrow::Borrow,
    boxed::Box,
    fmt,
    sync::{Arc, Weak},
};

pub type ObjectTypeStrongRef = Arc<ObjectType>;
pub type ObjectTypeWeakRef = Weak<ObjectType>;

pub type InputObjectTypeStrongRef = Arc<InputObjectType>;
pub type InputObjectTypeWeakRef = Weak<InputObjectType>;

pub type QuerySchemaRef = Arc<QuerySchema>;
pub type OutputTypeRef = Arc<OutputType>;
pub type OutputFieldRef = Arc<OutputField>;
pub type InputFieldRef = Arc<InputField>;
pub type EnumTypeRef = Arc<EnumType>;

/// The query schema.
/// Defines which operations (query/mutations) are possible on a database, based on the (internal) data model.
///
/// Conceptually, a query schema stores two trees (query / mutation) that consist of
/// input and output types. Special consideration is required when dealing with object types.
///
/// Object types can be referenced multiple times throughout the schema, also recursively, which requires the use
/// of weak references to prevent memory leaks. To simplify the overall management of Arcs and weaks, the
/// query schema is subject to a number of invariants.
/// The most important one is that the only strong references (Arc) to a single object types
/// is only ever held by the top-level QuerySchema struct, never by the trees, which only ever hold weak refs.
///
/// Using a QuerySchema should never involve dealing with the strong references.
#[derive(Debug)]
pub struct QuerySchema {
    pub query: OutputTypeRef,
    pub mutation: OutputTypeRef,

    /// Stores all strong refs to the input object types.
    input_object_types: Vec<InputObjectTypeStrongRef>,

    /// Stores all strong refs to the output object types.
    output_object_types: Vec<ObjectTypeStrongRef>,

    pub internal_data_model: InternalDataModelRef,
}

impl QuerySchema {
    pub fn new(
        query: OutputTypeRef,
        mutation: OutputTypeRef,
        input_object_types: Vec<InputObjectTypeStrongRef>,
        output_object_types: Vec<ObjectTypeStrongRef>,
        internal_data_model: InternalDataModelRef,
    ) -> Self {
        QuerySchema {
            query,
            mutation,
            input_object_types,
            output_object_types,
            internal_data_model,
        }
    }

    pub fn find_mutation_field<T>(&self, name: T) -> Option<OutputFieldRef>
    where
        T: Into<String>,
    {
        let name = name.into();
        self.mutation().get_fields().iter().find(|f| f.name == name).cloned()
    }

    pub fn find_query_field<T>(&self, name: T) -> Option<OutputFieldRef>
    where
        T: Into<String>,
    {
        let name = name.into();
        self.query().get_fields().iter().find(|f| f.name == name).cloned()
    }

    pub fn mutation(&self) -> ObjectTypeStrongRef {
        match self.mutation.borrow() {
            OutputType::Object(ref o) => o.into_arc(),
            _ => unreachable!(),
        }
    }

    pub fn query(&self) -> ObjectTypeStrongRef {
        match self.query.borrow() {
            OutputType::Object(ref o) => o.into_arc(),
            _ => unreachable!(),
        }
    }
}

pub struct ObjectType {
    name: String,

    fields: OnceCell<Vec<OutputFieldRef>>,

    // Object types can directly map to models.
    model: Option<ModelRef>,
}

impl Debug for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObjectType")
            .field("name", &self.name)
            .field("fields", &"#Fields Cell#")
            .field("model", &self.model)
            .finish()
    }
}

impl ObjectType {
    pub fn new<T>(name: T, model: Option<ModelRef>) -> Self
    where
        T: Into<String>,
    {
        Self {
            name: name.into(),
            fields: OnceCell::new(),
            model,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn get_fields(&self) -> &Vec<OutputFieldRef> {
        self.fields.get().unwrap()
    }

    pub fn set_fields(&self, fields: Vec<OutputField>) {
        self.fields.set(fields.into_iter().map(Arc::new).collect()).unwrap();
    }

    pub fn find_field(&self, name: &str) -> Option<OutputFieldRef> {
        self.get_fields().iter().find(|f| &f.name == name).cloned()
    }

    /// True if fields are empty, false otherwise.
    pub fn is_empty(&self) -> bool {
        self.get_fields().is_empty()
    }
}

#[derive(Debug)]
pub struct OutputField {
    pub name: String,
    pub field_type: OutputTypeRef,

    /// Arguments are input fields, but positioned in context of an output field
    /// instead of being attached to an input object.
    pub arguments: Vec<InputFieldRef>,

    /// Indicates if the presence of the field on the higher output objects.
    /// As opposed to input fields, optional output fields are also automatically nullable.
    pub is_required: bool,

    /// Relevant for resolving top level queries.
    pub query_info: Option<QueryInfo>,
}

impl OutputField {
    pub fn optional(mut self) -> Self {
        self.is_required = false;
        self
    }

    pub fn optional_if(self, condition: bool) -> Self {
        if condition {
            self.optional()
        } else {
            self
        }
    }
}

pub type QueryBuilderFn = dyn (Fn(ModelRef, ParsedField) -> QueryGraphBuilderResult<QueryGraph>) + Send + Sync;

/// Designates a specific top-level operation on a corresponding model.
#[derive(Debug)]
pub struct QueryInfo {
    pub model: Option<ModelRef>,
    pub tag: QueryTag,
}

/// A `QueryTag` designates a top level query possible with Prisma.
#[derive(Debug, Clone, PartialEq)]
pub enum QueryTag {
    FindOne,
    FindFirst,
    FindMany,
    CreateOne,
    UpdateOne,
    UpdateMany,
    DeleteOne,
    DeleteMany,
    UpsertOne,
    Aggregate,
    ExecuteRaw,
    QueryRaw,
}

impl fmt::Display for QueryTag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Self::FindOne => "findOne",
            Self::FindFirst => "findFirst",
            Self::FindMany => "findMany",
            Self::CreateOne => "createOne",
            Self::UpdateOne => "updateOne",
            Self::UpdateMany => "updateMany",
            Self::DeleteOne => "deleteOne",
            Self::DeleteMany => "deleteMany",
            Self::UpsertOne => "upsertOne",
            Self::Aggregate => "aggregate",
            Self::ExecuteRaw => "executeRaw",
            Self::QueryRaw => "queryRaw",
        };

        write!(f, "{}", s)
    }
}

#[derive(PartialEq)]
pub struct InputObjectType {
    pub name: String,
    pub constraints: InputObjectTypeConstraints,
    pub fields: OnceCell<Vec<InputFieldRef>>,
}

#[derive(Debug, Default, PartialEq)]
pub struct InputObjectTypeConstraints {
    /// The maximum number of fields that can be provided.
    pub min_num_fields: Option<usize>,

    /// The minimum number of fields that must be provided.
    pub max_num_fields: Option<usize>,
}

impl Debug for InputObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InputObjectType")
            .field("name", &self.name)
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
            .expect("InputObjectType::set_fields");
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

    /// Allow exactly one field of the possible ones to be in the input.
    pub fn require_exactly_one_field(&mut self) {
        self.set_max_fields(1);
        self.set_min_fields(1);
    }

    /// Allow at most one field of the possible ones to be in the input.
    pub fn allow_at_most_one_field(&mut self) {
        self.set_max_fields(1);
        self.set_min_fields(0);
    }

    /// Allow a maximum of `max` fields to be present in the input.
    pub fn set_max_fields(&mut self, max: usize) {
        self.constraints.max_num_fields = Some(max);
    }

    /// Require a minimum of `min` fields to be present in the input.
    pub fn set_min_fields(&mut self, min: usize) {
        self.constraints.min_num_fields = Some(min);
    }
}

#[derive(Debug, PartialEq)]
pub struct InputField {
    pub name: String,
    pub default_value: Option<dml::DefaultValue>,

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
}

#[derive(Clone)]
pub enum InputType {
    Scalar(ScalarType),
    Enum(EnumTypeRef),
    List(Box<InputType>),
    Object(InputObjectTypeWeakRef),
}

impl PartialEq for InputType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (InputType::Scalar(st), InputType::Scalar(ost)) => st.eq(ost),
            (InputType::Enum(_), InputType::Enum(_)) => true,
            (InputType::List(lt), InputType::List(olt)) => lt.eq(olt),
            (InputType::Object(obj), InputType::Object(oobj)) => obj.into_arc().name == oobj.into_arc().name,
            _ => false,
        }
    }
}

impl Debug for InputType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputType::Object(obj) => write!(f, "Object({})", obj.into_arc().name),
            InputType::Scalar(s) => write!(f, "{:?}", s),
            InputType::Enum(e) => write!(f, "{:?}", e),
            InputType::List(l) => write!(f, "{:?}", l),
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

    pub fn enum_type(containing: EnumTypeRef) -> InputType {
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
}

#[derive(Debug, Clone)]
pub enum OutputType {
    Enum(EnumTypeRef),
    List(OutputTypeRef),
    Object(ObjectTypeWeakRef),
    Scalar(ScalarType),
}

impl OutputType {
    pub fn list(containing: OutputType) -> OutputType {
        OutputType::List(Arc::new(containing))
    }

    pub fn object(containing: ObjectTypeWeakRef) -> OutputType {
        OutputType::Object(containing)
    }

    pub fn string() -> OutputType {
        OutputType::Scalar(ScalarType::String)
    }

    pub fn int() -> OutputType {
        OutputType::Scalar(ScalarType::Int)
    }

    pub fn bigint() -> OutputType {
        OutputType::Scalar(ScalarType::BigInt)
    }

    pub fn float() -> OutputType {
        OutputType::Scalar(ScalarType::Float)
    }

    pub fn decimal() -> OutputType {
        OutputType::Scalar(ScalarType::Decimal)
    }

    pub fn boolean() -> OutputType {
        OutputType::Scalar(ScalarType::Boolean)
    }

    pub fn date_time() -> OutputType {
        OutputType::Scalar(ScalarType::DateTime)
    }

    pub fn json() -> OutputType {
        OutputType::Scalar(ScalarType::Json)
    }

    pub fn uuid() -> OutputType {
        OutputType::Scalar(ScalarType::UUID)
    }

    pub fn xml() -> OutputType {
        OutputType::Scalar(ScalarType::Xml)
    }

    pub fn bytes() -> OutputType {
        OutputType::Scalar(ScalarType::Bytes)
    }

    /// Attempts to recurse through the type until an object type is found.
    /// Returns Some(ObjectTypeStrongRef) if ab object type is found, None otherwise.
    pub fn as_object_type(&self) -> Option<ObjectTypeStrongRef> {
        match self {
            OutputType::Enum(_) => None,
            OutputType::List(inner) => inner.as_object_type(),
            OutputType::Object(obj) => Some(obj.into_arc()),
            OutputType::Scalar(_) => None,
        }
    }

    pub fn is_list(&self) -> bool {
        matches!(self, OutputType::List(_))
    }

    pub fn is_object(&self) -> bool {
        matches!(self, OutputType::Object(_))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScalarType {
    Null,
    String,
    Int,
    BigInt,
    Float,
    Decimal,
    Boolean,
    Enum(EnumTypeRef),
    DateTime,
    Json,
    JsonList,
    UUID,
    Xml,
    Bytes,
}

impl From<EnumType> for OutputType {
    fn from(e: EnumType) -> Self {
        OutputType::Enum(Arc::new(e))
    }
}

impl From<EnumType> for InputType {
    fn from(e: EnumType) -> Self {
        InputType::Enum(Arc::new(e))
    }
}
