pub mod args {
    pub const WHERE: &str = "where";
    pub const DATA: &str = "data";

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

    // raw specific args
    pub const QUERY: &str = "query";
    pub const PARAMETERS: &str = "parameters";

    pub const DISTINCT: &str = "distinct";

    // createMany-specific args
    pub const SKIP_DUPLICATES: &str = "skipDuplicates";
}

pub mod operations {
    // nested operations
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

    // scalar lists
    pub const PUSH: &str = "push";

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
    pub const SEARCH: &str = "search";
    pub const LOWER_THAN: &str = "lt";
    pub const LOWER_THAN_OR_EQUAL: &str = "lte";
    pub const GREATER_THAN: &str = "gt";
    pub const GREATER_THAN_OR_EQUAL: &str = "gte";
    pub const IN: &str = "in";

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
    pub const ASC: &str = "asc";
    pub const DESC: &str = "desc";
}

pub mod output_fields {
    pub const AFFECTED_COUNT: &str = "count";
}

pub mod deprecation {
    pub const AGGR_DEPRECATION: &str =
        "Aggregation keywords got unified to use underscore as prefix to prevent field clashes.";
}
