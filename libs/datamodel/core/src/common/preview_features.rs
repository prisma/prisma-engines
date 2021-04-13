use lazy_static::lazy_static;
use serde::Serialize;
use PreviewFeature::*;

// Mapping of which active, deprecated and hidden
// features are in which place in the datamodel.
lazy_static! {
    /// Generator preview features
    static ref GENERATOR: FeatureMap = {
        FeatureMap::default().with_active(vec![
            SqlServer,
            OrderByRelation,
            NApi,
            SelectRelationCount,
            OrderByAggregationGroup
        ]).with_hidden(vec![
            MongoDb
        ]).with_deprecated(vec![
            AtomicNumberOperations,
            AggregateApi,
            Middlewares,
            NativeTypes,
            Distinct,
            ConnectOrCreate,
            TransactionApi,
            UncheckedScalarInputs,
            GroupBy,
            CreateMany
        ])
    };

    /// Datasource preview features.
    static ref DATASOURCE: FeatureMap = {
        FeatureMap::default()
    };
}

#[derive(Debug, Default)]
pub struct FeatureMap {
    /// Valid, visible features.
    active: Vec<PreviewFeature>,

    /// Deprecated features.
    deprecated: Vec<PreviewFeature>,

    /// Hidden preview features are valid features, but are not propagated into the tooling
    /// (as autocomplete or similar) or into error messages (eg. showing a list of valid features).
    hidden: Vec<PreviewFeature>,
}

impl FeatureMap {
    pub fn with_active(mut self, active: Vec<PreviewFeature>) -> Self {
        self.active = active;
        self
    }

    pub fn with_hidden(mut self, hidden: Vec<PreviewFeature>) -> Self {
        self.hidden = hidden;
        self
    }

    pub fn with_deprecated(mut self, deprecated: Vec<PreviewFeature>) -> Self {
        self.deprecated = deprecated;
        self
    }

    pub fn is_valid(&self, flag: &PreviewFeature) -> bool {
        self.active.contains(flag) || self.hidden.contains(flag)
    }

    pub fn is_deprecated(&self, flag: &PreviewFeature) -> bool {
        self.deprecated.contains(flag)
    }
}

/// (Usually) Append-only list of features.
#[derive(Debug, Copy, Clone, PartialEq, Serialize)]
pub enum PreviewFeature {
    ConnectOrCreate,
    TransactionApi,
    NativeTypes,
    GroupBy,
    CreateMany,
    AtomicNumberOperations,
    AggregateApi,
    Middlewares,
    Distinct,
    UncheckedScalarInputs,
    SqlServer,
    MongoDb,
    OrderByRelation,
    NApi,
    SelectRelationCount,
    OrderByAggregationGroup,
}

impl PreviewFeature {
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s.to_lowercase().as_str() {
            "connectorcreate" => Self::ConnectOrCreate,
            "transactionapi" => Self::TransactionApi,
            "nativetypes" => Self::NativeTypes,
            "groupby" => Self::GroupBy,
            "createmany" => Self::CreateMany,
            "atomicnumberoperations" => Self::AtomicNumberOperations,
            "aggregateapi" => Self::AggregateApi,
            "middlewares" => Self::Middlewares,
            "distinct" => Self::Distinct,
            "uncheckedscalarinputs" => Self::UncheckedScalarInputs,
            "sqlserver" => Self::SqlServer,
            "mongodb" => Self::MongoDb,
            "orderbyrelation" => Self::OrderByRelation,
            "napi" => Self::NApi,
            "selectrelationcount" => Self::SelectRelationCount,
            "orderbyaggregationgroup" => Self::OrderByAggregationGroup,
            _ => return None,
        })
    }
}

// // generator preview features
// const CONNECT_OR_CREATE: &str = "connectOrCreate";
// const TRANSACTION_API: &str = "transactionApi";
// const NATIVE_TYPES: &str = "nativeTypes";
// const SQL_SERVER: &str = "microsoftSqlServer";
// const MONGODB: &str = "mongodb";
// const GROUP_BY: &str = "groupBy";
// const CREATE_MANY: &str = "createMany";
// const ORDER_BY_RELATION: &str = "orderByRelation";
// const NAPI: &str = "napi";
// const SELECT_RELATION_COUNT: &str = "selectRelationCount";
// const ORDER_BY_AGGREGATE_GROUP: &str = "orderByAggregateGroup";

// // deprecated preview features
// const ATOMIC_NUMBER_OPERATIONS: &str = "atomicNumberOperations";
// const AGGREGATE_API: &str = "aggregateApi";
// const MIDDLEWARES: &str = "middlewares";
// const DISTINCT: &str = "distinct";
// const UNCHECKED_SCALAR_INPUTS: &str = "uncheckedScalarInputs";

// pub const DATASOURCE_PREVIEW_FEATURES: &[&str] = &[];

// pub const GENERATOR_PREVIEW_FEATURES: &[&str] = &[
//     SQL_SERVER,
//     ORDER_BY_RELATION,
//     NAPI,
//     SELECT_RELATION_COUNT,
//     ORDER_BY_AGGREGATE_GROUP,
// ];

// /// Hidden preview features are valid features, but are not propagated into the tooling
// /// (as autocomplete or similar) or into error messages (eg. showing a list of valid features).
// pub const HIDDEN_GENERATOR_PREVIEW_FEATURES: &[&str] = &[MONGODB];

// pub const DEPRECATED_GENERATOR_PREVIEW_FEATURES: &[&str] = &[
//     ATOMIC_NUMBER_OPERATIONS,
//     AGGREGATE_API,
//     MIDDLEWARES,
//     NATIVE_TYPES,
//     DISTINCT,
//     CONNECT_OR_CREATE,
//     TRANSACTION_API,
//     UNCHECKED_SCALAR_INPUTS,
//     GROUP_BY,
//     CREATE_MANY,
// ];

// pub const DEPRECATED_DATASOURCE_PREVIEW_FEATURES: &[&str] = &[];

// /// Returns all current features available.
// pub fn current_features() -> Vec<&'static str> {
//     let mut features = Vec::from(GENERATOR_PREVIEW_FEATURES);
//     features.extend(HIDDEN_GENERATOR_PREVIEW_FEATURES);
//     features
// }
