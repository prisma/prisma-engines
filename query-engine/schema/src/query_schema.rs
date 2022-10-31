use super::*;
use fmt::Debug;
use prisma_models::{psl::PreviewFeature, InternalDataModelRef, ModelRef};
use psl::datamodel_connector::{ConnectorCapability, RelationMode};
use std::{borrow::Borrow, fmt};

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
    /// Root query object (read queries).
    pub query: OutputTypeRef,

    /// Root mutation object (write queries).
    pub mutation: OutputTypeRef,

    /// Internal abstraction over the datamodel AST.
    pub internal_data_model: InternalDataModelRef,

    /// Information about the connector this schema was build for.
    pub context: ConnectorContext,

    /// Internal. Stores all strong Arc refs to the input object types.
    _input_object_types: Vec<InputObjectTypeStrongRef>,

    /// Internal. Stores all strong Arc refs to the output object types.
    _output_object_types: Vec<ObjectTypeStrongRef>,

    /// Internal. Stores all enum refs.
    _enum_types: Vec<EnumTypeRef>,
}

/// Connector meta information, to be used in query execution if necessary.
#[derive(Debug)]
pub struct ConnectorContext {
    /// Capabilities of the provider.
    pub capabilities: Vec<ConnectorCapability>,

    /// Enabled preview features.
    pub features: Vec<PreviewFeature>,

    /// Relation mode of the provider
    pub relation_mode: RelationMode,
}

impl ConnectorContext {
    pub fn new(
        capabilities: Vec<ConnectorCapability>,
        features: Vec<PreviewFeature>,
        relation_mode: RelationMode,
    ) -> Self {
        Self {
            capabilities,
            features,
            relation_mode,
        }
    }

    pub fn can_native_upsert(&self) -> bool {
        self.capabilities.contains(&ConnectorCapability::NativeUpsert)
    }
}

impl QuerySchema {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        query: OutputTypeRef,
        mutation: OutputTypeRef,
        _input_object_types: Vec<InputObjectTypeStrongRef>,
        _output_object_types: Vec<ObjectTypeStrongRef>,
        _enum_types: Vec<EnumTypeRef>,
        internal_data_model: InternalDataModelRef,
        capabilities: Vec<ConnectorCapability>,
        features: Vec<PreviewFeature>,
        relation_mode: RelationMode,
    ) -> Self {
        QuerySchema {
            query,
            mutation,
            _input_object_types,
            _output_object_types,
            _enum_types,
            internal_data_model,
            context: ConnectorContext::new(capabilities, features, relation_mode),
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

    pub fn enum_types(&self) -> &[EnumTypeRef] {
        &self._enum_types
    }

    pub fn context(&self) -> &ConnectorContext {
        &self.context
    }
}

/// Designates a specific top-level operation on a corresponding model.
#[derive(Debug)]
pub struct QueryInfo {
    pub model: Option<ModelRef>,
    pub tag: QueryTag,
}

/// A `QueryTag` designates a top level query possible with Prisma.
#[derive(Debug, Clone, PartialEq)]
pub enum QueryTag {
    FindUnique,
    FindFirst,
    FindMany,
    CreateOne,
    CreateMany,
    UpdateOne,
    UpdateMany,
    DeleteOne,
    DeleteMany,
    UpsertOne,
    Aggregate,
    GroupBy,
    ExecuteRaw,
    QueryRaw { query_type: Option<String> },
}

impl fmt::Display for QueryTag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Self::FindUnique => "findUnique",
            Self::FindFirst => "findFirst",
            Self::FindMany => "findMany",
            Self::CreateOne => "createOne",
            Self::CreateMany => "createMany",
            Self::UpdateOne => "updateOne",
            Self::UpdateMany => "updateMany",
            Self::DeleteOne => "deleteOne",
            Self::DeleteMany => "deleteMany",
            Self::UpsertOne => "upsertOne",
            Self::Aggregate => "aggregate",
            Self::GroupBy => "groupBy",
            Self::ExecuteRaw => "executeRaw",
            Self::QueryRaw { query_type: _ } => "queryRaw",
        };

        write!(f, "{}", s)
    }
}

#[derive(PartialEq, Hash, Eq, Debug, Clone)]
pub struct Identifier {
    name: String,
    namespace: String,
}

impl Identifier {
    pub fn new<T, U>(name: T, namespace: U) -> Self
    where
        T: Into<String>,
        U: Into<String>,
    {
        Self {
            name: name.into(),
            namespace: namespace.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }
}

impl ToString for Identifier {
    fn to_string(&self) -> String {
        format!("{}.{}", self.namespace(), self.name())
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
    DateTime,
    Json,
    JsonList,
    UUID,
    Xml,
    Bytes,
}

impl std::fmt::Display for ScalarType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let typ = match self {
            ScalarType::Null => "Null",
            ScalarType::String => "String",
            ScalarType::Int => "Int",
            ScalarType::BigInt => "BigInt",
            ScalarType::Boolean => "Boolean",
            ScalarType::Float => "Float",
            ScalarType::Decimal => "Decimal",
            ScalarType::DateTime => "DateTime",
            ScalarType::Json => "Json",
            ScalarType::UUID => "UUID",
            ScalarType::JsonList => "Json",
            ScalarType::Xml => "Xml",
            ScalarType::Bytes => "Bytes",
        };

        write!(f, "{}", typ)
    }
}
