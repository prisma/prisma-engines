use serde::{Serialize, Serializer};
use std::fmt;

/// A set of preview features.
pub type PreviewFeatures = enumflags2::BitFlags<PreviewFeature>;

macro_rules! features {
    ($( $variant:ident $(,)? ),*) => {
        #[enumflags2::bitflags]
        #[repr(u64)]
        #[derive(Debug, Copy, Clone, PartialEq, Eq)]
        pub enum PreviewFeature {
            $( $variant,)*
        }

        impl PreviewFeature {
            pub fn parse_opt(s: &str) -> Option<Self> {
                $(
                    if s.eq_ignore_ascii_case(stringify!($variant)) { return Some(Self::$variant) }
                )*

                None
            }
        }

        impl fmt::Display for PreviewFeature {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let variant = match self { $( Self::$variant => stringify!($variant),)* };
                let mut first_char = variant.chars().next().unwrap();
                first_char.make_ascii_lowercase();
                f.write_fmt(format_args!("{first_char}{rest}", rest = &variant[1..]))
            }
        }
    };
}

// (Usually) Append-only list of features.
features!(
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
    MicrosoftSqlServer,
    OrderByRelation,
    MongoDb,
    NApi,
    SelectRelationCount,
    OrderByAggregateGroup,
    FilterJson,
    ReferentialIntegrity,
    ReferentialActions,
    InteractiveTransactions,
    NamedConstraints,
    FullTextSearch,
    FullTextIndex,
    DataProxy,
    ExtendedIndexes,
    Cockroachdb,
    Tracing,
    ImprovedQueryRaw,
    Metrics,
    OrderByNulls,
    MultiSchema,
    FilteredRelationCount,
    FieldReference,
    PostgresqlExtensions,
    ClientExtensions,
    Deno,
    ExtendedWhereUnique,
    Views,
    JsonProtocol,
    NodeDrivers,
);

/// Generator preview features
pub const ALL_PREVIEW_FEATURES: FeatureMap = FeatureMap {
    active: enumflags2::make_bitflags!(PreviewFeature::{
        Deno
         | FullTextSearch
         | FullTextIndex
         | Tracing
         | Metrics
         | FieldReference
         | PostgresqlExtensions
         | ExtendedWhereUnique
         | MultiSchema
         | Views
    }),
    deprecated: enumflags2::make_bitflags!(PreviewFeature::{
        AtomicNumberOperations
        | AggregateApi
        | Cockroachdb
        | ExtendedIndexes
        | FilterJson
        | Middlewares
        | NamedConstraints
        | NativeTypes
        | Distinct
        | ConnectOrCreate
        | TransactionApi
        | UncheckedScalarInputs
        | GroupBy
        | CreateMany
        | MicrosoftSqlServer
        | SelectRelationCount
        | MongoDb
        | OrderByAggregateGroup
        | OrderByRelation
        | ReferentialActions
        | ReferentialIntegrity
        | NApi
        | ImprovedQueryRaw
        | DataProxy
        | InteractiveTransactions
        | ClientExtensions
        | FilteredRelationCount
        | OrderByNulls
        | JsonProtocol
    }),
    hidden: enumflags2::make_bitflags!(PreviewFeature::{
        NodeDrivers
    }),
};

#[derive(Debug)]
pub struct FeatureMap {
    /// Valid, visible features.
    active: PreviewFeatures,

    /// Deprecated features.
    deprecated: PreviewFeatures,

    /// Hidden preview features are valid features, but are not propagated into the tooling
    /// (as autocomplete or similar) or into error messages (eg. showing a list of valid features).
    hidden: PreviewFeatures,
}

impl FeatureMap {
    pub const fn active_features(&self) -> PreviewFeatures {
        self.active
    }

    pub const fn hidden_features(&self) -> PreviewFeatures {
        self.hidden
    }

    pub(crate) fn is_valid(&self, flag: PreviewFeature) -> bool {
        (self.active | self.hidden).contains(flag)
    }

    pub(crate) fn is_deprecated(&self, flag: PreviewFeature) -> bool {
        self.deprecated.contains(flag)
    }
}

impl Serialize for PreviewFeature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
