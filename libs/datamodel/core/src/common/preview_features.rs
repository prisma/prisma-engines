// datasource preview features
const NATIVE_TYPES: &str = "nativeTypes";
const SQL_SERVER: &str = "microsoftSqlServer";

// generator preview features
const ATOMIC_NUMBER_OPERATIONS: &str = "atomicNumberOperations";
const CONNECT_OR_CREATE: &str = "connectOrCreate";
const TRANSACTION_API: &str = "transactionApi";

// deprecated preview features
const AGGREGATE_API: &str = "aggregateApi"; // todo move this to deprecated preview features list for VSCode
const MIDDLEWARES: &str = "middlewares";
const DISTINCT: &str = "distinct";

pub const DATASOURCE_PREVIEW_FEATURES: &[&'static str] = &[NATIVE_TYPES, SQL_SERVER];
pub const GENERATOR_PREVIEW_FEATURES: &[&'static str] = &[
    ATOMIC_NUMBER_OPERATIONS,
    CONNECT_OR_CREATE,
    TRANSACTION_API,
    AGGREGATE_API,
    MIDDLEWARES,
    DISTINCT,
];

pub const DEPRECATED_GENERATOR_PREVIEW_FEATURES: &[&'static str] = &[AGGREGATE_API, MIDDLEWARES, DISTINCT];
