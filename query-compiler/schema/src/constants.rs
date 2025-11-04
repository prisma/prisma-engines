pub mod args {
    pub const WHERE: &str = "where";
    pub const DATA: &str = "data";
    pub const RELATION_LOAD_STRATEGY: &str = "relationLoadStrategy";

    // upsert args
    pub const CREATE: &str = "create";
    pub const UPDATE: &str = "update";

    // pagination args
    pub const CURSOR: &str = "cursor";
    pub const TAKE: &str = "take";
    pub const SKIP: &str = "skip";

    // sorting args
    pub const ORDER_BY: &str = "orderBy";

    // aggregation args
    pub const BY: &str = "by";
    pub const HAVING: &str = "having";

    pub const DISTINCT: &str = "distinct";

    // createMany-specific args
    pub const SKIP_DUPLICATES: &str = "skipDuplicates";

    // deleteMany-specific args
    pub const LIMIT: &str = "limit";
}

pub mod operations {
    // nested operations and composites
    pub const CONNECT: &str = "connect";
    pub const CREATE: &str = "create";
    pub const CREATE_MANY: &str = "createMany";
    pub const CONNECT_OR_CREATE: &str = "connectOrCreate";
    pub const DISCONNECT: &str = "disconnect";
    pub const UPDATE: &str = "update";
    pub const UPDATE_MANY: &str = "updateMany";
    pub const DELETE: &str = "delete";
    pub const DELETE_MANY: &str = "deleteMany";
    pub const UPSERT: &str = "upsert";
    pub const SET: &str = "set";

    // scalar lists and composites
    pub const PUSH: &str = "push";
    pub const UNSET: &str = "unset";

    // numbers
    pub const INCREMENT: &str = "increment";
    pub const DECREMENT: &str = "decrement";
    pub const MULTIPLY: &str = "multiply";
    pub const DIVIDE: &str = "divide";
}

pub mod filters {
    // scalar filters
    pub const EQUALS: &str = "equals";
    pub const CONTAINS: &str = "contains";
    pub const STARTS_WITH: &str = "startsWith";
    pub const ENDS_WITH: &str = "endsWith";
    pub const LOWER_THAN: &str = "lt";
    pub const LOWER_THAN_OR_EQUAL: &str = "lte";
    pub const GREATER_THAN: &str = "gt";
    pub const GREATER_THAN_OR_EQUAL: &str = "gte";
    pub const IN: &str = "in";
    pub const SEARCH: &str = "search";
    pub const IS_SET: &str = "isSet";
    pub const UNDERSCORE_REF: &str = "_ref";
    pub const UNDERSCORE_CONTAINER: &str = "_container";

    // legacy filter
    pub const NOT_IN: &str = "notIn";

    // case-sensitivity filters
    pub const MODE: &str = "mode";
    pub const INSENSITIVE: &str = "insensitive";
    pub const DEFAULT: &str = "default";

    // condition filters
    pub const AND: &str = "AND";
    pub const AND_LOWERCASE: &str = "and";
    pub const OR: &str = "OR";
    pub const OR_LOWERCASE: &str = "or";
    pub const NOT: &str = "NOT";
    pub const NOT_LOWERCASE: &str = "not";

    // List-specific filters
    pub const HAS: &str = "has";
    pub const HAS_NONE: &str = "hasNone";
    pub const HAS_SOME: &str = "hasSome";
    pub const HAS_EVERY: &str = "hasEvery";
    pub const IS_EMPTY: &str = "isEmpty";

    // m2m filters
    pub const EVERY: &str = "every";
    pub const SOME: &str = "some";
    pub const NONE: &str = "none";

    // o2m filters
    pub const IS: &str = "is";
    pub const IS_NOT: &str = "isNot";

    // json filters
    pub const PATH: &str = "path";
    pub const ARRAY_CONTAINS: &str = "array_contains";
    pub const ARRAY_STARTS_WITH: &str = "array_starts_with";
    pub const ARRAY_ENDS_WITH: &str = "array_ends_with";
    pub const STRING_CONTAINS: &str = "string_contains";
    pub const STRING_STARTS_WITH: &str = "string_starts_with";
    pub const STRING_ENDS_WITH: &str = "string_ends_with";
    pub const JSON_TYPE: &str = "json_type";
}

pub mod aggregations {
    pub const UNDERSCORE_COUNT: &str = "_count";
    pub const UNDERSCORE_AVG: &str = "_avg";
    pub const UNDERSCORE_SUM: &str = "_sum";
    pub const UNDERSCORE_MIN: &str = "_min";
    pub const UNDERSCORE_MAX: &str = "_max";

    pub const COUNT: &str = "count";
    pub const AVG: &str = "avg";
    pub const SUM: &str = "sum";
    pub const MIN: &str = "min";
    pub const MAX: &str = "max";
}

pub mod ordering {
    pub const SORT_ORDER: &str = "SortOrder";
    pub const NULLS_ORDER: &str = "NullsOrder";
    pub const ASC: &str = "asc";
    pub const DESC: &str = "desc";
    pub const FIRST: &str = "first";
    pub const LAST: &str = "last";

    // Full-text-search specifics
    pub const UNDERSCORE_RELEVANCE: &str = "_relevance";
    pub const SEARCH: &str = "search";
    pub const SORT: &str = "sort";
    pub const NULLS: &str = "nulls";
    pub const FIELDS: &str = "fields";
}

pub mod json_null {
    /// Name of the enum used for filter inputs.
    pub const FILTER_ENUM_NAME: &str = "JsonNullValueFilter";

    /// Name of the enum used for write inputs.
    pub const INPUT_ENUM_NAME: &str = "JsonNullValueInput";

    /// Name of the enum used for write inputs, nullable field.
    pub const NULLABLE_INPUT_ENUM_NAME: &str = "NullableJsonNullValueInput";

    pub const DB_NULL: &str = "DbNull";
    pub const JSON_NULL: &str = "JsonNull";
    pub const ANY_NULL: &str = "AnyNull";
}

pub mod output_fields {
    pub const AFFECTED_COUNT: &str = "count";
}

pub mod itx {
    pub const READ_UNCOMMITTED: &str = "ReadUncommitted";
    pub const READ_COMMITTED: &str = "ReadCommitted";
    pub const REPEATABLE_READ: &str = "RepeatableRead";
    pub const SERIALIZABLE: &str = "Serializable";
    pub const SNAPSHOT: &str = "Snapshot";
}

pub mod deprecation {}

pub mod load_strategy {
    pub const JOIN: &str = "join";
    pub const QUERY: &str = "query";
}
