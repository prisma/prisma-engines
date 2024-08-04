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

// (Usually) Append-only list of features. (alphabetically sorted)
features!(
    AggregateApi,
    AtomicNumberOperations,
    ClientExtensions,
    Cockroachdb,
    ConnectOrCreate,
    CreateMany,
    DataProxy,
    Deno,
    Distinct,
    DriverAdapters,
    ExtendedIndexes,
    ExtendedWhereUnique,
    FieldReference,
    FilteredRelationCount,
    FilterJson,
    FullTextIndex,
    FullTextSearch,
    GroupBy,
    ImprovedQueryRaw,
    InteractiveTransactions,
    JsonProtocol,
    Metrics,
    MicrosoftSqlServer,
    Middlewares,
    MongoDb,
    MultiSchema,
    NamedConstraints,
    NApi,
    NativeDistinct,
    NativeTypes,
    OrderByAggregateGroup,
    OrderByNulls,
    OrderByRelation,
    PostgresqlExtensions,
    ReferentialActions,
    ReferentialIntegrity,
    SelectRelationCount,
    Tracing,
    TransactionApi,
    UncheckedScalarInputs,
    Views,
    RelationJoins,
    ReactNative,
    PrismaSchemaFolder,
    OmitApi
);

/// Generator preview features (alphabetically sorted)
pub const ALL_PREVIEW_FEATURES: FeatureMap = FeatureMap {
    active: enumflags2::make_bitflags!(PreviewFeature::{
        Deno
         | DriverAdapters
         | FullTextIndex
         | FullTextSearch
         | Metrics
         | MultiSchema
         | NativeDistinct
         | PostgresqlExtensions
         | Tracing
         | Views
         | RelationJoins
         | OmitApi
         | PrismaSchemaFolder
    }),
    deprecated: enumflags2::make_bitflags!(PreviewFeature::{
        AtomicNumberOperations
        | AggregateApi
        | ClientExtensions
        | Cockroachdb
        | ConnectOrCreate
        | CreateMany
        | DataProxy
        | Distinct
        | ExtendedIndexes
        | ExtendedWhereUnique
        | FieldReference
        | FilteredRelationCount
        | FilterJson
        | GroupBy
        | ImprovedQueryRaw
        | InteractiveTransactions
        | JsonProtocol
        | MicrosoftSqlServer
        | Middlewares
        | MongoDb
        | NamedConstraints
        | NApi
        | NativeTypes
        | OrderByAggregateGroup
        | OrderByNulls
        | OrderByRelation
        | ReferentialActions
        | ReferentialIntegrity
        | SelectRelationCount
        | TransactionApi
        | UncheckedScalarInputs
    }),
    hidden: enumflags2::make_bitflags!(PreviewFeature::{ReactNative}),
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
