use lazy_static::lazy_static;
use paste::paste;
use serde::{Serialize, Serializer};
use PreviewFeature::*;

macro_rules! features {
    ($( $variant:ident $(,)? ),*) => {
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
        pub enum PreviewFeature {
            $(
                $variant,
            )*
        }

        impl PreviewFeature {
            pub fn parse_opt(s: &str) -> Option<Self> {
                paste! {
                    let parsed = match s.to_lowercase().as_str() {
                        $(
                            stringify!([<$variant:lower>]) => Self::$variant,
                        )*
                        _ => return None,
                    };
                }

                Some(parsed)
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
);

// Mapping of which active, deprecated and hidden
// features are valid in which place in the datamodel.
lazy_static! {
    /// Generator preview features
    pub static ref GENERATOR: FeatureMap = {
        FeatureMap::default().with_active(vec![
            MicrosoftSqlServer,
            OrderByRelation,
            NApi,
            SelectRelationCount,
            OrderByAggregateGroup,
            FilterJson,
            PlanetScaleMode,
        ]).with_hidden(vec![
            MongoDb
        ]).with_deprecated(vec![
            AtomicNumberOperations,
            AggregateApi,
            Middlewares,
            NativeTypes,
            Distinct,
            ConnectOrCreate,
            TransactionApi,
            UncheckedScalarInputs,
            GroupBy,
            CreateMany
        ])
    };

    /// Datasource preview features.
    pub static ref DATASOURCE: FeatureMap = {
        FeatureMap::default()
    };
}

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
    format!("{}{}", first_char.to_lowercase(), s[1..].to_owned())
}
