use std::{fmt, str::FromStr};

/// Not all Databases are created equal. Hence connectors for our datasources support different capabilities.
/// These are used during schema validation. E.g. if a connector does not support enums an error will be raised.
macro_rules! capabilities {
    ($( $variant:ident $(,)? ),*) => {
        #[derive(Debug, Clone, Copy, PartialEq)]
        #[enumflags2::bitflags]
        #[repr(u64)]
        pub enum ConnectorCapability {
            $(
                $variant,
            )*
        }

        impl fmt::Display for ConnectorCapability {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let name = match self {
                    $(
                        Self::$variant => stringify!($variant),
                    )*
                };

                f.write_str(name)
            }
        }

        impl FromStr for ConnectorCapability {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $(
                        stringify!($variant) => Ok(Self::$variant),
                    )*
                    _ => Err(format!("{} is not a known connector capability.", s)),
                }
            }
        }
    };
}

// Capabilities describe what functionality connectors are able to provide.
// Some are used only by the query engine, some are used only by the datamodel parser.
capabilities!(
    // General capabilities, not specific to any part of Prisma.
    ScalarLists,
    Enums,
    Json,
    JsonLists,
    AutoIncrement,
    RelationFieldsInArbitraryOrder,
    CompositeTypes,
    DefaultValueAuto,
    TwoWayEmbeddedManyToManyRelation,
    ImplicitManyToManyRelation,
    MultiSchema,
    //Start of ME/IE only capabilities
    AutoIncrementAllowedOnNonId,
    AutoIncrementMultipleAllowed,
    AutoIncrementNonIndexedAllowed,
    NamedPrimaryKeys,
    NamedForeignKeys,
    ReferenceCycleDetection,
    NamedDefaultValues,
    IndexColumnLengthPrefixing,
    PrimaryKeySortOrderDefinition,
    FullTextIndex,
    SortOrderInFullTextIndex,
    MultipleFullTextAttributesPerModel,
    ClusteringSetting,
    // Start of query-engine-only Capabilities
    EnumArrayPush, // implies the ScalarList capability. Necessary, as CockroachDB supports pushing to a list of scalars, but not to the particular case of an enum list. See https://github.com/cockroachdb/cockroach/issues/71388
    InsensitiveFilters,
    CreateMany,
    CreateManyWriteableAutoIncId,
    SupportsDefaultInInsert, // This capability is set if connector supports using `DEFAULT` instead of a value in the list of `INSERT` arguments.
    WritableAutoincField,
    CreateSkipDuplicates,
    UpdateableId,
    JsonFiltering, // Used as an umbrella in tests to filter for connectors that supports json filtering.
    JsonFilteringJsonPath, // Connector supports filtering json fields using json path (eg: mysql).
    JsonFilteringArrayPath, // Connector supports filtering json fields using array path (eg: postgres).
    JsonFilteringAlphanumeric, // Connector supports alphanumeric json filters (gt, gte, lt, lte...).
    JsonFilteringAlphanumericFieldRef, // Connector supports alphanumeric json filters against a json field reference.
    CompoundIds,
    AnyId, // Any (or combination of) uniques and not only id fields can constitute an id for a model.
    SqlQueryRaw,
    MongoDbQueryRaw,
    FullTextSearch,
    FullTextSearchWithoutIndex,
    FullTextSearchWithIndex,
    AdvancedJsonNullability,    // Connector distinguishes between their null type and JSON null.
    UndefinedType,              // Connector distinguishes `null` and `undefined`
    DecimalType,                // Connector supports Prisma Decimal type.
    BackwardCompatibleQueryRaw, // Temporary SQLite specific capability. Should be removed once https://github.com/prisma/prisma/issues/12784 is fixed,
    OrderByNullsFirstLast,      // Connector supports ORDER BY NULLS LAST/FIRST
    FilteredInlineChildNestedToOneDisconnect, // Connector supports a filtered nested disconnect on both sides of a to-one relation.
    // Block of isolation levels.
    SupportsTxIsolationReadUncommitted,
    SupportsTxIsolationReadCommitted,
    SupportsTxIsolationRepeatableRead,
    SupportsTxIsolationSerializable,
    SupportsTxIsolationSnapshot,
    NativeUpsert,
    InsertReturning,
    UpdateReturning,
    RowIn,                                  // Connector supports (a, b) IN (c, d) expression.
    DistinctOn,                             // Connector supports DB-level distinct (e.g. postgres)
    DeleteReturning,                        // Connector supports deleting records and returning them in one operation.
    SupportsFiltersOnRelationsWithoutJoins, // Connector supports rendering filters on relation fields without joins.
    LateralJoin,                            // Connector supports lateral joins to resolve relations.
    CorrelatedSubqueries,                   // Connector supports correlated subqueries to resolve relations.
);

/// Contains all capabilities that the connector is able to serve.
pub type ConnectorCapabilities = enumflags2::BitFlags<ConnectorCapability>;
