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
        self.mutation()
            .get_fields()
            .iter()
            .find(|f| f.name == name)
            .cloned()
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

    /// Arguments are input fields, but positioned in context of an output field
    /// instead of being attached to an input object.
    pub arguments: Vec<InputFieldRef>,
    pub field_type: OutputTypeRef,

    /// Indicates if the presence of the field on the higher output objects.
    /// As opposed to input fields, optional output fields are also automatically nullable.
    pub is_required: bool,
    pub query_builder: Option<SchemaQueryBuilder>,
}

impl OutputField {
    pub fn query_builder(&self) -> Option<&SchemaQueryBuilder> {
        self.query_builder.as_ref()
    }

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
pub struct ModelQueryBuilder {
    pub model: ModelRef,
    pub tag: QueryTag,

    /// An associated builder is responsible for building queries
    /// that the executer will execute. The result info is required
    /// by the serialization to correctly build the response.
    pub builder_fn: Box<QueryBuilderFn>,
}

impl Debug for ModelQueryBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModelQueryBuilder")
            .field("model", &self.model)
            .field("tag", &self.tag)
            .field("builder_fn", &"#BuilderFn")
            .finish()
    }
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
    FindFirst,
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
            QueryTag::FindFirst => "findFirst",
            QueryTag::FindMany => "findMany",
            QueryTag::CreateOne => "createOne",
            QueryTag::UpdateOne => "updateOne",
            QueryTag::UpdateMany => "updateMany",
            QueryTag::DeleteOne => "deleteOne",
            QueryTag::DeleteMany => "deleteMany",
            QueryTag::UpsertOne => "upsertOne",
            QueryTag::Aggregate => "aggregate",
        };

        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone)]
pub struct GenericQueryBuilder {
    // WIP
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

    pub fn null() -> InputType {
        InputType::Scalar(ScalarType::Null)
    }

    pub fn enum_type(containing: EnumTypeRef) -> InputType {
        InputType::Enum(containing)
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
    Float,
    Boolean,
    Enum(EnumTypeRef),
    DateTime,
    Json,
    JsonList,
    UUID,
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
