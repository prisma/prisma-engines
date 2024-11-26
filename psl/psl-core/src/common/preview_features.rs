use serde::{Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::LazyLock;

/// A set of preview features.
pub type PreviewFeatures = enumflags2::BitFlags<PreviewFeature>;

macro_rules! features {
    ($( $variant:ident $(,)? ),*) => {
        #[enumflags2::bitflags]
        #[repr(u64)]
        #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
    NativeFullTextSearchPostgres,
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
    OmitApi,
    TypedSql,
    StrictUndefinedChecks
);

#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
struct RenamedFeatureKey {
    /// The old, deprecated preview feature that was renamed.
    pub from: PreviewFeature,

    /// The provider that the feature was renamed for.
    pub provider: Option<&'static str>,
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct RenamedFeatureValue {
    /// The new preview feature.
    pub to: PreviewFeature,

    /// The Pris.ly link endpoint for the feature, i.e., what comes after `https://pris.ly/d/`.
    pub prisly_link_endpoint: &'static str,
}

#[derive(Debug, Clone)]
pub(crate) enum RenamedFeature {
    /// The preview feature was renamed for a specific provider.
    ForProvider((&'static str, RenamedFeatureValue)),

    /// The preview feature was renamed for all providers.
    AllProviders(RenamedFeatureValue),
}

#[derive(Debug, Clone)]
struct FeatureMap {
    /// Valid, visible features.
    active: PreviewFeatures,

    /// Valid, but connector-specific features that are only visible on matching provider key.
    native: BTreeMap<&'static str, PreviewFeatures>,

    /// Deprecated features.
    deprecated: PreviewFeatures,

    /// History of renamed deprecated features.
    renamed: BTreeMap<RenamedFeatureKey, RenamedFeatureValue>,

    /// Hidden preview features are valid features, but are not propagated into the tooling
    /// (as autocomplete or similar) or into error messages (eg. showing a list of valid features).
    hidden: PreviewFeatures,
}

#[derive(Debug, Clone)]
pub struct FeatureMapWithProvider {
    provider: Option<&'static str>,
    feature_map: FeatureMap,
}

/// The default feature map with an unknown provider.
/// This is used for convenience in `prisma/language-tools`, which needs the list of all available preview features
/// before a provider is necessarily known.
pub static ALL_PREVIEW_FEATURES: LazyLock<FeatureMapWithProvider> = LazyLock::new(|| FeatureMapWithProvider::new(None));

impl FeatureMapWithProvider {
    pub fn new(connector_provider: Option<&'static str>) -> FeatureMapWithProvider {
        // Generator preview features (alphabetically sorted)
        let feature_map: FeatureMap = FeatureMap {
            active: enumflags2::make_bitflags!(PreviewFeature::{
                Deno
                 | DriverAdapters
                 | Metrics
                 | MultiSchema
                 | NativeDistinct
                 | OmitApi
                 | PostgresqlExtensions
                 | PrismaSchemaFolder
                 | RelationJoins
                 | StrictUndefinedChecks
                 | Tracing
                 | Views
            }),
            native: BTreeMap::from([
                #[cfg(feature = "postgresql")]
                (
                    "postgresql",
                    enumflags2::make_bitflags!(PreviewFeature::{
                        NativeFullTextSearchPostgres
                    }),
                ),
            ]),
            renamed: BTreeMap::from([
                #[cfg(feature = "postgresql")]
                (
                    RenamedFeatureKey {
                        from: PreviewFeature::FullTextSearch,
                        provider: Some("postgresql"),
                    },
                    RenamedFeatureValue {
                        to: PreviewFeature::NativeFullTextSearchPostgres,
                        prisly_link_endpoint: "native-fts-postgres",
                    },
                ),
            ]),
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
                | FullTextIndex
                | FullTextSearch
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
            hidden: enumflags2::make_bitflags!(PreviewFeature::{ReactNative | TypedSql}),
        };

        Self {
            provider: connector_provider,
            feature_map,
        }
    }

    pub fn native_features(&self) -> PreviewFeatures {
        self.provider
            .and_then(|provider| self.feature_map.native.get(provider).copied())
            .unwrap_or_default()
    }

    pub fn active_features(&self) -> PreviewFeatures {
        self.feature_map.active | self.native_features()
    }

    pub const fn hidden_features(&self) -> PreviewFeatures {
        self.feature_map.hidden
    }

    pub(crate) fn is_valid(&self, flag: PreviewFeature) -> bool {
        (self.active_features() | self.feature_map.hidden).contains(flag)
    }

    pub(crate) fn is_deprecated(&self, flag: PreviewFeature) -> bool {
        self.feature_map.deprecated.contains(flag)
    }

    /// Was the given preview feature deprecated and renamed?
    pub(crate) fn is_renamed<'f>(&self, flag: PreviewFeature) -> Option<RenamedFeature> {
        // Check for a renamed feature specific to the provider. This is only possible if a provider is not None.
        let provider_specific = self.provider.and_then(|provider| {
            self.feature_map
                .renamed
                .get(&RenamedFeatureKey {
                    from: flag,
                    provider: Some(provider),
                })
                .map(|renamed| RenamedFeature::ForProvider((provider, renamed.clone())))
        });

        // Fallback to provider-independent renamed feature
        provider_specific.or_else(|| {
            self.feature_map
                .renamed
                .get(&RenamedFeatureKey {
                    from: flag,
                    provider: None,
                })
                .map(|renamed| RenamedFeature::AllProviders(renamed.clone()))
        })
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
