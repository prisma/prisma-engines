use once_cell::sync::Lazy;
use serde::{Serialize, Serializer};
use PreviewFeature::*;

macro_rules! features {
    ($( $variant:ident $(,)? ),*) => {
        #[enumflags2::bitflags]
        #[repr(u32)]
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
        pub enum PreviewFeature {
            $(
                $variant,
            )*
        }

        impl PreviewFeature {
            pub fn parse_opt(s: &str) -> Option<Self> {
                $(
                    if s.eq_ignore_ascii_case(stringify!($variant)) { return Some(Self::$variant) }
                )*

                None
            }
        }

        impl ToString for PreviewFeature {
            fn to_string(&self) -> String {
                match self {
                    $(
                        Self::$variant => decapitalize(stringify!($variant)),
                    )*
                }
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
    MongoDb,
    OrderByRelation,
    NApi,
    SelectRelationCount,
    OrderByAggregateGroup,
    FilterJson,
    PlanetScaleMode,
    ReferentialActions,
);

// Mapping of which active, deprecated and hidden
// features are valid in which place in the datamodel.

/// Generator preview features
pub static GENERATOR: Lazy<FeatureMap> = Lazy::new(|| {
    FeatureMap::default()
        .with_active(vec![
            MicrosoftSqlServer,
            OrderByRelation,
            NApi,
            SelectRelationCount,
            OrderByAggregateGroup,
            FilterJson,
            PlanetScaleMode,
            ReferentialActions,
            MongoDb,
        ])
        .with_deprecated(vec![
            AtomicNumberOperations,
            AggregateApi,
            Middlewares,
            NativeTypes,
            Distinct,
            ConnectOrCreate,
            TransactionApi,
            UncheckedScalarInputs,
            GroupBy,
            CreateMany,
        ])
});

/// Datasource preview features.
pub static DATASOURCE: Lazy<FeatureMap> = Lazy::new(FeatureMap::default);

#[derive(Debug, Default)]
pub struct FeatureMap {
    /// Valid, visible features.
    active: Vec<PreviewFeature>,

    /// Deprecated features.
    deprecated: Vec<PreviewFeature>,

    /// Hidden preview features are valid features, but are not propagated into the tooling
    /// (as autocomplete or similar) or into error messages (eg. showing a list of valid features).
    hidden: Vec<PreviewFeature>,
}

impl FeatureMap {
    pub fn active_features(&self) -> &[PreviewFeature] {
        &self.active
    }

    pub fn hidden_features(&self) -> &[PreviewFeature] {
        &self.hidden
    }

    fn with_active(mut self, active: Vec<PreviewFeature>) -> Self {
        self.active = active;
        self
    }

    #[allow(dead_code)]
    fn with_hidden(mut self, hidden: Vec<PreviewFeature>) -> Self {
        self.hidden = hidden;
        self
    }

    fn with_deprecated(mut self, deprecated: Vec<PreviewFeature>) -> Self {
        self.deprecated = deprecated;
        self
    }

    pub fn is_valid(&self, flag: &PreviewFeature) -> bool {
        self.active.contains(flag) || self.hidden.contains(flag)
    }

    pub fn is_deprecated(&self, flag: &PreviewFeature) -> bool {
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

/// Lowercases first character.
/// Assumes 1-byte characters!
pub fn decapitalize(s: &str) -> String {
    let first_char = s.chars().next().unwrap();
    format!("{}{}", first_char.to_lowercase(), &s[1..])
}
