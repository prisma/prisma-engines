use crate::{IdentifierType, ObjectType, OutputField};
use prisma_models::{ast, InternalDataModel};
use psl::{
    datamodel_connector::{Connector, ConnectorCapability, RelationMode},
    PreviewFeature, PreviewFeatures,
};
use std::{collections::HashMap, fmt};

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
enum Operation {
    Query,
    Mutation,
}

type LazyField = Box<dyn for<'a> Fn(&'a QuerySchema) -> OutputField<'a> + Send + Sync>;

/// The query schema defines which operations (query/mutations) are possible on a database, based
/// on a Prisma schema.
///
/// Conceptually, a query schema stores two trees (query/mutation) that consist of input and output
/// types.
pub struct QuerySchema {
    /// Internal abstraction over the datamodel AST.
    pub internal_data_model: InternalDataModel,

    pub(crate) enable_raw_queries: bool,
    pub(crate) connector: &'static dyn Connector,

    /// Indexes query and mutation fields by their own query info for easier access.
    query_info_map: HashMap<(Operation, QueryInfo), usize>,
    root_fields: HashMap<(Operation, String), usize>,
    query_fields: Vec<LazyField>,
    mutation_fields: Vec<LazyField>,

    preview_features: PreviewFeatures,

    /// Relation mode in the datasource.
    relation_mode: RelationMode,
}

impl QuerySchema {
    pub(crate) fn new(
        enable_raw_queries: bool,
        connector: &'static dyn Connector,
        preview_features: PreviewFeatures,
        internal_data_model: InternalDataModel,
    ) -> Self {
        let relation_mode = internal_data_model.schema.relation_mode();

        let mut query_schema = QuerySchema {
            preview_features,
            enable_raw_queries,
            query_info_map: Default::default(),
            root_fields: Default::default(),
            internal_data_model,
            connector,
            relation_mode,
            mutation_fields: Default::default(),
            query_fields: Default::default(),
        };

        query_schema.query_fields = crate::build::query_type::query_fields(&query_schema);
        query_schema.mutation_fields = crate::build::mutation_type::mutation_fields(&query_schema);

        let mut query_info_map: HashMap<(Operation, QueryInfo), _> = HashMap::new();
        let mut root_fields = HashMap::new();

        for (idx, field_fn) in query_schema.query_fields.iter().enumerate() {
            let field = field_fn(&query_schema);
            if let Some(query_info) = field.query_info() {
                query_info_map.insert((Operation::Query, query_info.to_owned()), idx);
            }
            root_fields.insert((Operation::Query, field.name.into_owned()), idx);
        }

        for (idx, field_fn) in query_schema.mutation_fields.iter().enumerate() {
            let field = field_fn(&query_schema);
            if let Some(query_info) = field.query_info() {
                query_info_map.insert((Operation::Mutation, query_info.to_owned()), idx);
            }
            root_fields.insert((Operation::Mutation, field.name.into_owned()), idx);
        }

        query_schema.query_info_map = query_info_map;
        query_schema.root_fields = root_fields;
        query_schema
    }

    pub(crate) fn supports_any(&self, capabilities: &[ConnectorCapability]) -> bool {
        capabilities.iter().any(|c| self.connector.has_capability(*c))
    }

    pub(crate) fn can_full_text_search(&self) -> bool {
        self.has_feature(PreviewFeature::FullTextSearch)
            && (self.has_capability(ConnectorCapability::FullTextSearchWithoutIndex)
                || self.has_capability(ConnectorCapability::FullTextSearchWithIndex))
    }

    pub(crate) fn has_feature(&self, feature: PreviewFeature) -> bool {
        self.preview_features.contains(feature)
    }

    pub(crate) fn has_capability(&self, capability: ConnectorCapability) -> bool {
        self.connector.has_capability(capability)
    }

    pub fn find_mutation_field(&self, name: &str) -> Option<OutputField<'_>> {
        self.root_fields
            .get(&(Operation::Mutation, name.to_owned()))
            .map(|i| self.mutation_fields[*i](self))
    }

    pub fn find_query_field(&self, name: &str) -> Option<OutputField<'_>> {
        self.root_fields
            .get(&(Operation::Query, name.to_owned()))
            .map(|i| self.query_fields[*i](self))
    }

    pub fn find_query_field_by_model_and_action(
        &self,
        model_name: Option<&str>,
        tag: QueryTag,
    ) -> Option<OutputField<'_>> {
        let model = model_name
            .and_then(|name| self.internal_data_model.schema.db.find_model(name))
            .map(|m| m.id);
        let query_info = QueryInfo { model, tag };

        self.query_info_map
            .get(&(Operation::Query, query_info))
            .map(|i| self.query_fields[*i](self))
    }

    pub fn find_mutation_field_by_model_and_action(
        &self,
        model_name: Option<&str>,
        tag: QueryTag,
    ) -> Option<OutputField<'_>> {
        let model = model_name
            .and_then(|name| self.internal_data_model.schema.db.find_model(name))
            .map(|m| m.id);
        let query_info = QueryInfo { model, tag };

        self.query_info_map
            .get(&(Operation::Mutation, query_info))
            .map(|i| self.mutation_fields[*i](self))
    }

    pub fn mutation(&self) -> ObjectType<'_> {
        ObjectType::new(Identifier::new_prisma(IdentifierType::Mutation), || {
            self.mutation_fields.iter().map(|f| f(self)).collect()
        })
    }

    pub fn query(&self) -> ObjectType<'_> {
        ObjectType::new(Identifier::new_prisma(IdentifierType::Query), || {
            self.query_fields.iter().map(|f| f(self)).collect()
        })
    }

    pub fn relation_mode(&self) -> RelationMode {
        self.relation_mode
    }

    pub fn can_native_upsert(&self) -> bool {
        self.connector
            .capabilities()
            .contains(ConnectorCapability::NativeUpsert)
    }
}

/// Designates a specific top-level operation on a corresponding model.
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct QueryInfo {
    pub model: Option<ast::ModelId>,
    pub tag: QueryTag,
}

/// A `QueryTag` designates a top level query possible with Prisma.
#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub enum QueryTag {
    FindUnique,
    FindUniqueOrThrow,
    FindFirst,
    FindFirstOrThrow,
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
    // Raw operations
    ExecuteRaw,
    QueryRaw,
    RunCommandRaw,
    FindRaw,
    AggregateRaw,
}

impl fmt::Display for QueryTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::FindUnique => "findUnique",
            Self::FindUniqueOrThrow => "findUniqueOrThrow",
            Self::FindFirst => "findFirst",
            Self::FindFirstOrThrow => "findFirstOrThrow",
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
            Self::QueryRaw => "queryRaw",
            Self::RunCommandRaw => "runCommandRaw",
            Self::FindRaw => "findRaw",
            Self::AggregateRaw => "aggregateRaw",
        };

        f.write_str(s)
    }
}

impl From<&str> for QueryTag {
    fn from(value: &str) -> Self {
        match value {
            "findUnique" => Self::FindUnique,
            "findUniqueOrThrow" => Self::FindUniqueOrThrow,
            "findFirst" => Self::FindFirst,
            "findFirstOrThrow" => Self::FindFirstOrThrow,
            "findMany" => Self::FindMany,
            "createOne" => Self::CreateOne,
            "createMany" => Self::CreateMany,
            "updateOne" => Self::UpdateOne,
            "updateMany" => Self::UpdateMany,
            "deleteOne" => Self::DeleteOne,
            "deleteMany" => Self::DeleteMany,
            "upsertOne" => Self::UpsertOne,
            "aggregate" => Self::Aggregate,
            "groupBy" => Self::GroupBy,
            "executeRaw" => Self::ExecuteRaw,
            "queryRaw" => Self::QueryRaw,
            "findRaw" => Self::FindRaw,
            "aggregateRaw" => Self::AggregateRaw,
            "runCommandRaw" => Self::RunCommandRaw,
            v => panic!("Unknown query tag: {v}"),
        }
    }
}

#[derive(PartialEq, Hash, Eq, Debug, Clone)]
pub struct Identifier {
    name: IdentifierType,
    namespace: IdentifierNamespace,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
enum IdentifierNamespace {
    Prisma,
    Model,
}

impl Identifier {
    pub(crate) fn new_prisma(name: impl Into<IdentifierType>) -> Self {
        Self {
            name: name.into(),
            namespace: IdentifierNamespace::Prisma,
        }
    }

    pub(crate) fn new_model(name: impl Into<IdentifierType>) -> Self {
        Self {
            name: name.into(),
            namespace: IdentifierNamespace::Model,
        }
    }

    pub fn name(&self) -> String {
        self.name.to_string()
    }

    pub fn namespace(&self) -> &str {
        match self.namespace {
            IdentifierNamespace::Prisma => "prisma",
            IdentifierNamespace::Model => "model",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Copy)]
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

impl fmt::Display for ScalarType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

        f.write_str(typ)
    }
}
