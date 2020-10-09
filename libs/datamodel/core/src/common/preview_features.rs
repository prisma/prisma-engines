// datasource preview features
const NATIVE_TYPES: &str = "nativeTypes";

// generator preview features
const ATOMIC_NUMBER_OPERATIONS: &str = "atomicNumberOperations";
const CONNECT_OR_CREATE: &str = "connectOrCreate";
const TRANSACTION_API: &str = "transactionApi";

// deprecated preview features
const AGGREGATE_API: &str = "aggregateApi"; // todo move this to deprecated preview features list for VSCode
const MIDDLEWARES: &str = "middlewares";
const DISTINCT: &str = "distinct";

pub const DATASOURCE_PREVIEW_FEATURES: [&str; 1] = [NATIVE_TYPES];
pub const GENERATOR_PREVIEW_FEATURES: [&str; 6] = [
    ATOMIC_NUMBER_OPERATIONS,
    CONNECT_OR_CREATE,
    TRANSACTION_API,
    AGGREGATE_API,
    MIDDLEWARES,
    DISTINCT,
];
