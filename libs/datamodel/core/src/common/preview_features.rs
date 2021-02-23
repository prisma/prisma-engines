// datasource preview features

// generator preview features
const CONNECT_OR_CREATE: &str = "connectOrCreate";
const TRANSACTION_API: &str = "transactionApi";
const NATIVE_TYPES: &str = "nativeTypes";
const SQL_SERVER: &str = "microsoftSqlServer";
const GROUP_BY: &str = "groupBy";
const CREATE_MANY: &str = "createMany";
const ORDER_BY_RELATION: &str = "orderByRelation";
const NAPI: &str = "napi";

// deprecated preview features
const ATOMIC_NUMBER_OPERATIONS: &str = "atomicNumberOperations";
const AGGREGATE_API: &str = "aggregateApi";
const MIDDLEWARES: &str = "middlewares";
const DISTINCT: &str = "distinct";
const UNCHECKED_SCALAR_INPUTS: &str = "uncheckedScalarInputs";

pub const DATASOURCE_PREVIEW_FEATURES: &[&str] = &[];

pub const GENERATOR_PREVIEW_FEATURES: &[&str] = &[SQL_SERVER, GROUP_BY, CREATE_MANY, ORDER_BY_RELATION, NAPI];

pub const DEPRECATED_GENERATOR_PREVIEW_FEATURES: &[&str] = &[
    ATOMIC_NUMBER_OPERATIONS,
    AGGREGATE_API,
    MIDDLEWARES,
    NATIVE_TYPES,
    DISTINCT,
    CONNECT_OR_CREATE,
    TRANSACTION_API,
    UNCHECKED_SCALAR_INPUTS,
];

pub const DEPRECATED_DATASOURCE_PREVIEW_FEATURES: &[&str] = &[];
