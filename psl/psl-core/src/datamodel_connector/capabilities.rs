use std::{fmt, str::FromStr};

/// Not all Databases are created equal. Hence connectors for our datasources support different capabilities.
/// These are used during schema validation. E.g. if a connector does not support enums an error will be raised.
macro_rules! capabilities {
    ($( $variant:ident $(,)? ),*) => {
        #[derive(Debug, Clone, Copy, PartialEq)]
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

                write!(f, "{}", name)
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
    EnumArrayPush,
    InsensitiveFilters,
    CreateMany,
    CreateManyWriteableAutoIncId,
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
    FullTextSearchWithoutIndex,
    FullTextSearchWithIndex,
    AdvancedJsonNullability,    // Connector distinguishes between their null type and JSON null.
    UndefinedType,              // Connector distinguishes `null` and `undefined`
    DecimalType,                // Connector supports Prisma Decimal type.
    BackwardCompatibleQueryRaw, // Temporary SQLite specific capability. Should be removed once https://github.com/prisma/prisma/issues/12784 is fixed,
    OrderByNullsFirstLast,      // Connector supports ORDER BY NULLS LAST/FIRST
    // Block of isolation levels.
    SupportsTxIsolationReadUncommitted,
    SupportsTxIsolationReadCommitted,
    SupportsTxIsolationRepeatableRead,
    SupportsTxIsolationSerializable,
    SupportsTxIsolationSnapshot,
    NativeUpsert
);

/// Contains all capabilities that the connector is able to serve.
#[derive(Debug)]
pub struct ConnectorCapabilities {
    pub capabilities: Vec<ConnectorCapability>,
}

impl ConnectorCapabilities {
    pub fn empty() -> Self {
        Self { capabilities: vec![] }
    }

    pub fn new(capabilities: Vec<ConnectorCapability>) -> Self {
        Self { capabilities }
    }

    pub fn contains(&self, capability: ConnectorCapability) -> bool {
        self.capabilities.contains(&capability)
    }

    pub fn supports_any(&self, capabilities: &[ConnectorCapability]) -> bool {
        self.capabilities
            .iter()
            .any(|connector_capability| capabilities.contains(connector_capability))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_cap_does_not_contain() {
        let cap = ConnectorCapabilities::empty();
        assert!(!cap.supports_any(&[ConnectorCapability::JsonFilteringJsonPath]));
    }

    #[test]
    fn test_cap_with_others_does_not_contain() {
        let cap = ConnectorCapabilities::new(vec![
            ConnectorCapability::PrimaryKeySortOrderDefinition,
            ConnectorCapability::JsonFilteringArrayPath,
        ]);
        assert!(!cap.supports_any(&[ConnectorCapability::JsonFilteringJsonPath]));
    }

    #[test]
    fn test_cap_with_others_does_contain() {
        let cap = ConnectorCapabilities::new(vec![
            ConnectorCapability::PrimaryKeySortOrderDefinition,
            ConnectorCapability::JsonFilteringJsonPath,
            ConnectorCapability::JsonFilteringArrayPath,
        ]);
        assert!(cap.supports_any(&[
            ConnectorCapability::JsonFilteringJsonPath,
            ConnectorCapability::JsonFilteringArrayPath,
        ]));
    }

    #[test]
    fn test_does_contain() {
        let cap = ConnectorCapabilities::new(vec![
            ConnectorCapability::PrimaryKeySortOrderDefinition,
            ConnectorCapability::JsonFilteringArrayPath,
        ]);
        assert!(!cap.supports_any(&[ConnectorCapability::JsonFilteringJsonPath]));
    }
}
