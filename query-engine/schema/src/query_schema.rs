use crate::{IdentifierType, ObjectType, OutputField};
use psl::{
    can_support_relation_load_strategy,
    datamodel_connector::{Connector, ConnectorCapabilities, ConnectorCapability, JoinStrategySupport, RelationMode},
    has_capability, parser_database as db, PreviewFeature, PreviewFeatures,
};
use query_structure::InternalDataModel;
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

    /// Whether the database supports `RelationLoadStrategy::Join`.
    /// By the time the `QuerySchema`` is created, we don't have all the evidence yet to determine
    /// whether the database supports the join strategy (eg: database version).
    // Hack: Ideally, this shoud be known statically and live in the PSL connector entirely.
    join_strategy_support: JoinStrategySupport,
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
            join_strategy_support: if preview_features.contains(PreviewFeature::RelationJoins)
                && can_support_relation_load_strategy()
            {
                connector.runtime_join_strategy_support()
            } else {
                JoinStrategySupport::No
            },
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
        capabilities.iter().any(|c| has_capability(self.connector, *c))
    }

    pub(crate) fn can_full_text_search(&self) -> bool {
        self.has_feature(PreviewFeature::FullTextSearch) && self.has_capability(ConnectorCapability::FullTextSearch)
    }

    /// Returns whether the loaded connector supports the join strategy.
    pub fn can_resolve_relation_with_joins(&self) -> bool {
        !matches!(self.join_strategy_support(), JoinStrategySupport::No)
    }

    /// Returns whether the database version of the loaded connector supports the join strategy.
    pub fn join_strategy_support(&self) -> JoinStrategySupport {
        if !can_support_relation_load_strategy() {
            return JoinStrategySupport::No;
        }
        self.join_strategy_support
    }

    /// Augments the join strategy support with the runtime database version knowledge.
    /// This is specifically designed for the MySQL connector, which does not support the join strategy for versions < 8.0.14 and MariaDB.
    pub fn with_db_version_supports_join_strategy(self, db_version_supports_joins_strategy: bool) -> Self {
        let augmented_support = match self.join_strategy_support {
            JoinStrategySupport::UnknownYet => match db_version_supports_joins_strategy {
                true => JoinStrategySupport::Yes,
                false => JoinStrategySupport::UnsupportedDbVersion,
            },
            x => x,
        };

        Self {
            join_strategy_support: augmented_support,
            ..self
        }
    }

    pub fn has_feature(&self, feature: PreviewFeature) -> bool {
        self.preview_features.contains(feature)
    }

    pub fn has_capability(&self, capability: ConnectorCapability) -> bool {
        has_capability(self.connector, capability)
    }

    pub fn capabilities(&self) -> ConnectorCapabilities {
        self.connector.capabilities()
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
    pub model: Option<db::ModelId>,
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
    CreateManyAndReturn,
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
            Self::CreateManyAndReturn => "createManyAndReturn",
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
            "createManyAndReturn" => Self::CreateManyAndReturn,
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
            ScalarType::Bytes => "Bytes",
        };

        f.write_str(typ)
    }
}
