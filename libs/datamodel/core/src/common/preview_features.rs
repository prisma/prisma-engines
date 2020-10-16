// datasource preview features

// generator preview features
const ATOMIC_NUMBER_OPERATIONS: &str = "atomicNumberOperations";
const CONNECT_OR_CREATE: &str = "connectOrCreate";
const TRANSACTION_API: &str = "transactionApi";
const NATIVE_TYPES: &str = "nativeTypes";
const SQL_SERVER: &str = "microsoftSqlServer";

// deprecated preview features
const AGGREGATE_API: &str = "aggregateApi";
const MIDDLEWARES: &str = "middlewares";
const DISTINCT: &str = "distinct";

pub const DATASOURCE_PREVIEW_FEATURES: &[&'static str] = &[];
pub const GENERATOR_PREVIEW_FEATURES: &[&'static str] = &[
    ATOMIC_NUMBER_OPERATIONS,
    CONNECT_OR_CREATE,
    TRANSACTION_API,
    NATIVE_TYPES,
    SQL_SERVER,
];

pub const DEPRECATED_GENERATOR_PREVIEW_FEATURES: &[&'static str] = &[AGGREGATE_API, MIDDLEWARES, DISTINCT];
pub const DEPRECATED_DATASOURCE_PREVIEW_FEATURES: &[&'static str] = &[];
