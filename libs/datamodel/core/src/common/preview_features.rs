// datasource preview features

// generator preview features
const CONNECT_OR_CREATE: &str = "connectOrCreate";
const TRANSACTION_API: &str = "transactionApi";
const NATIVE_TYPES: &str = "nativeTypes";
const SQL_SERVER: &str = "microsoftSqlServer";
const UNCHECKED_SCALAR_INPUTS: &str = "uncheckedScalarInputs";

// deprecated preview features
const ATOMIC_NUMBER_OPERATIONS: &str = "atomicNumberOperations";
const AGGREGATE_API: &str = "aggregateApi";
const MIDDLEWARES: &str = "middlewares";
const DISTINCT: &str = "distinct";

pub const DATASOURCE_PREVIEW_FEATURES: &[&'static str] = &[];

pub const GENERATOR_PREVIEW_FEATURES: &[&'static str] = &[NATIVE_TYPES, SQL_SERVER, UNCHECKED_SCALAR_INPUTS];

pub const DEPRECATED_GENERATOR_PREVIEW_FEATURES: &[&'static str] = &[
    ATOMIC_NUMBER_OPERATIONS,
    AGGREGATE_API,
    MIDDLEWARES,
    DISTINCT,
    CONNECT_OR_CREATE,
    TRANSACTION_API,
];

pub const DEPRECATED_DATASOURCE_PREVIEW_FEATURES: &[&'static str] = &[];
