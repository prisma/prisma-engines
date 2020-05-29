use super::*;
use crate::{ParsedField, QueryGraph, QueryGraphBuilderResult};
use once_cell::sync::OnceCell;
use prisma_models::{dml, InternalDataModelRef, ModelRef, TypeHint};
use std::{
    borrow::Borrow,
    boxed::Box,
    fmt,
    sync::{Arc, Weak},
};

pub type OutputTypeRef = Arc<OutputType>;

pub type ObjectTypeStrongRef = Arc<ObjectType>;
pub type ObjectTypeRef = Weak<ObjectType>;

pub type InputObjectTypeStrongRef = Arc<InputObjectType>;
pub type InputObjectTypeRef = Weak<InputObjectType>;

pub type QuerySchemaRef = Arc<QuerySchema>;
pub type FieldRef = Arc<Field>;
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

    pub fn find_mutation_field<T>(&self, name: T) -> Option<FieldRef>
    where
        T: Into<String>,
    {
        let name = name.into();
        self.mutation()
            .get_fields()
            .into_iter()
            .find(|f| f.name == name)
            .cloned()
    }

    pub fn find_query_field<T>(&self, name: T) -> Option<FieldRef>
    where
        T: Into<String>,
    {
        let name = name.into();
        self.query().get_fields().into_iter().find(|f| f.name == name).cloned()
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

#[derive(DebugStub)]
pub struct ObjectType {
    name: String,

    #[debug_stub = "#Fields Cell#"]
    fields: OnceCell<Vec<FieldRef>>,

    // Object types can directly map to models.
    model: Option<ModelRef>,
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

    pub fn get_fields(&self) -> &Vec<FieldRef> {
        self.fields.get().unwrap()
    }

    pub fn set_fields(&self, fields: Vec<Field>) {
        self.fields.set(fields.into_iter().map(Arc::new).collect()).unwrap();
    }

    pub fn find_field(&self, name: &str) -> Option<FieldRef> {
        self.get_fields().into_iter().find(|f| &f.name == name).cloned()
    }

    /// True if fields are empty, false otherwise.
    pub fn is_empty(&self) -> bool {
        self.get_fields().is_empty()
    }
}

#[derive(Debug)]
pub struct Field {
    pub name: String,
    pub arguments: Vec<Argument>,
    pub field_type: OutputTypeRef,
    pub query_builder: Option<SchemaQueryBuilder>,
}

impl Field {
    pub fn query_builder(&self) -> Option<&SchemaQueryBuilder> {
        self.query_builder.as_ref()
    }
}

/// Todo rework description.
/// A query builder allows to attach queries to the schema:
/// on a field:
/// - A `ModelQueryBuilder` builds queries specific to models, such as `findOne<Model>`.
///   It requires additional context compared to a `GenericQueryBuilder`.
///
/// - A `GenericQueryBuilder` is a query builder that requires no additional context but
///   the parsed query document data from the incoming query and is thus not associated to any particular
///   model. The `ResetData` query is such an example.
#[derive(Debug)]
pub enum SchemaQueryBuilder {
    ModelQueryBuilder(ModelQueryBuilder),
    GenericQueryBuilder(GenericQueryBuilder),
}

impl SchemaQueryBuilder {
    pub fn build(&self, parsed_field: ParsedField) -> QueryGraphBuilderResult<QueryGraph> {
        match self {
            Self::ModelQueryBuilder(m) => m.build(parsed_field),
            _ => unimplemented!(),
        }
    }
}

pub type QueryBuilderFn = dyn (Fn(ModelRef, ParsedField) -> QueryGraphBuilderResult<QueryGraph>) + Send + Sync;

/// Designates a specific top-level operation on a corresponding model.
#[derive(DebugStub)]
pub struct ModelQueryBuilder {
    pub model: ModelRef,
    pub tag: QueryTag,

    /// An associated builder is responsible for building queries
    /// that the executer will execute. The result info is required
    /// by the serialization to correctly build the response.
    #[debug_stub = "#BuilderFn#"]
    pub builder_fn: Box<QueryBuilderFn>,
}

impl ModelQueryBuilder {
    pub fn new(model: ModelRef, tag: QueryTag, builder_fn: Box<QueryBuilderFn>) -> Self {
        Self { model, tag, builder_fn }
    }

    pub fn build(&self, parsed_field: ParsedField) -> QueryGraphBuilderResult<QueryGraph> {
        (self.builder_fn)(Arc::clone(&self.model), parsed_field)
    }
}

/// Designates top level model queries. Used for DMMF serialization.
#[derive(Debug, Clone, PartialEq)]
pub enum QueryTag {
    FindOne,
    FindMany,
    CreateOne,
    UpdateOne,
    UpdateMany,
    DeleteOne,
    DeleteMany,
    UpsertOne,
    Aggregate,
}

impl fmt::Display for QueryTag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            QueryTag::FindOne => "findOne",
            QueryTag::FindMany => "findMany",
            QueryTag::CreateOne => "createOne",
            QueryTag::UpdateOne => "updateOne",
            QueryTag::UpdateMany => "updateMany",
            QueryTag::DeleteOne => "deleteOne",
            QueryTag::DeleteMany => "deleteMany",
            QueryTag::UpsertOne => "upsertOne",
            QueryTag::Aggregate => "aggregate",
        };

        s.fmt(f)
    }
}

#[derive(Debug, Clone)]
pub struct GenericQueryBuilder {
    // WIP
}

#[derive(Debug)]
pub struct Argument {
    pub name: String,
    pub argument_type: InputType,
    pub default_value: Option<dml::DefaultValue>,
}

#[derive(DebugStub)]
pub struct InputObjectType {
    pub name: String,

    #[debug_stub = "#Input Fields Cell#"]
    pub fields: OnceCell<Vec<InputFieldRef>>,
}

impl InputObjectType {
    pub fn get_fields(&self) -> &Vec<InputFieldRef> {
        self.fields.get().unwrap()
    }

    pub fn set_fields(&self, fields: Vec<InputField>) {
        self.fields
            .set(fields.into_iter().map(|f| Arc::new(f)).collect())
            .unwrap();
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
        self.get_fields().into_iter().find(|f| f.name == name).cloned()
    }
}

#[derive(Debug)]
pub struct InputField {
    pub name: String,
    pub field_type: InputType,
    pub default_value: Option<dml::DefaultValue>,
}

#[derive(Debug, Clone)]
pub enum InputType {
    Scalar(ScalarType),
    Enum(EnumTypeRef),
    List(Box<InputType>),
    Object(InputObjectTypeRef),

    /// An optional input type may be provided, meaning only that the presence
    /// of the input is required or not, but doesn't make any assumption about
    /// whether or not the input can be null.
    Opt(Box<InputType>),

    /// A nullable input denotes that, if provided, a given input can be null.
    /// This makes no assumption about if an input needs to be provided or not.
    Null(Box<InputType>),
}

impl From<&InputType> for TypeHint {
    fn from(i: &InputType) -> Self {
        match i {
            InputType::Opt(inner) => (&**inner).into(),
            InputType::Null(inner) => (&**inner).into(),
            InputType::Scalar(st) => st.into(),
            InputType::Enum(_) => TypeHint::Enum,
            InputType::List(_) => TypeHint::Array,
            InputType::Object(_) => TypeHint::Unknown,
        }
    }
}

impl InputType {
    pub fn list(containing: InputType) -> InputType {
        InputType::List(Box::new(containing))
    }

    pub fn opt(containing: InputType) -> InputType {
        InputType::Opt(Box::new(containing))
    }

    pub fn null(containing: InputType) -> InputType {
        InputType::Null(Box::new(containing))
    }

    pub fn object(containing: InputObjectTypeRef) -> InputType {
        InputType::Object(containing)
    }

    pub fn string() -> InputType {
        InputType::Scalar(ScalarType::String)
    }

    pub fn int() -> InputType {
        InputType::Scalar(ScalarType::Int)
    }

    pub fn float() -> InputType {
        InputType::Scalar(ScalarType::Float)
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
}

#[derive(Debug)]
pub enum OutputType {
    Enum(EnumTypeRef),
    List(OutputTypeRef),
    Object(ObjectTypeRef),
    Opt(OutputTypeRef),
    Scalar(ScalarType),
}

impl OutputType {
    pub fn list(containing: OutputType) -> OutputType {
        OutputType::List(Arc::new(containing))
    }

    pub fn opt(containing: OutputType) -> OutputType {
        OutputType::Opt(Arc::new(containing))
    }

    pub fn object(containing: ObjectTypeRef) -> OutputType {
        OutputType::Object(containing)
    }

    pub fn string() -> OutputType {
        OutputType::Scalar(ScalarType::String)
    }

    pub fn int() -> OutputType {
        OutputType::Scalar(ScalarType::Int)
    }

    pub fn float() -> OutputType {
        OutputType::Scalar(ScalarType::Float)
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

    /// Attempts to recurse through the type until an object type is found.
    /// Returns Some(ObjectTypeStrongRef) if ab object type is found, None otherwise.
    pub fn as_object_type(&self) -> Option<ObjectTypeStrongRef> {
        match self {
            OutputType::Enum(_) => None,
            OutputType::List(inner) => inner.as_object_type(),
            OutputType::Object(obj) => Some(obj.into_arc()),
            OutputType::Opt(inner) => inner.as_object_type(),
            OutputType::Scalar(_) => None,
        }
    }

    pub fn is_list(&self) -> bool {
        match self {
            OutputType::Opt(inner) => inner.is_list(),
            OutputType::List(_) => true,
            _ => false,
        }
    }

    pub fn is_object(&self) -> bool {
        match self {
            OutputType::Opt(inner) => inner.is_object(),
            OutputType::Object(_) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ScalarType {
    String,
    Int,
    Float,
    Boolean,
    Enum(EnumTypeRef),
    DateTime,
    Json,
    JsonList,
    UUID,
}

impl From<&ScalarType> for TypeHint {
    fn from(t: &ScalarType) -> Self {
        match t {
            ScalarType::String => TypeHint::String,
            ScalarType::Int => TypeHint::Int,
            ScalarType::Float => TypeHint::Float,
            ScalarType::Boolean => TypeHint::Boolean,
            ScalarType::Enum(_) => TypeHint::Enum,
            ScalarType::DateTime => TypeHint::DateTime,
            ScalarType::Json => TypeHint::Json,
            ScalarType::JsonList => TypeHint::Json,
            ScalarType::UUID => TypeHint::UUID,
        }
    }
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
