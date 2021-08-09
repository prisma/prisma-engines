pub mod connector_error;
pub mod helper;

mod empty_connector;

pub use empty_connector::EmptyDatamodelConnector;

use crate::connector_error::{ConnectorError, ConnectorErrorFactory, ErrorKind};
use dml::{
    field::Field, model::Model, native_type_constructor::NativeTypeConstructor,
    native_type_instance::NativeTypeInstance, relation_info::ReferentialAction, scalars::ScalarType,
};
use enumflags2::BitFlags;
use std::{borrow::Cow, collections::BTreeMap, str::FromStr};

pub trait Connector: Send + Sync {
    fn name(&self) -> &str;

    fn capabilities(&self) -> &[ConnectorCapability];

    /// The maximum length of constraint names in bytes. Connectors without a
    /// limit should return usize::MAX.
    fn constraint_name_length(&self) -> usize;

    fn has_capability(&self, capability: ConnectorCapability) -> bool {
        self.capabilities().contains(&capability)
    }

    fn referential_actions(&self) -> BitFlags<ReferentialAction>;

    fn supports_named_primary_keys(&self) -> bool {
        self.has_capability(ConnectorCapability::NamedPrimaryKeys)
    }

    fn supports_named_foreign_keys(&self) -> bool {
        self.has_capability(ConnectorCapability::NamedForeignKeys)
    }

    fn supports_referential_action(&self, action: ReferentialAction) -> bool {
        self.referential_actions().contains(action)
    }

    fn emulates_referential_actions(&self) -> bool {
        false
    }

    fn validate_field(&self, field: &Field) -> Result<(), ConnectorError>;

    fn validate_model(&self, model: &Model) -> Result<(), ConnectorError>;

    /// Returns all available native type constructors available through this connector.
    /// Powers the auto completion of the vs code plugin.
    fn available_native_type_constructors(&self) -> &[NativeTypeConstructor];

    /// Returns the Scalar Type for the given native type
    fn scalar_type_for_native_type(&self, native_type: serde_json::Value) -> ScalarType;

    /// On each connector, each built-in Prisma scalar type (`Boolean`,
    /// `String`, `Float`, etc.) has a corresponding native type.
    fn default_native_type_for_scalar_type(&self, scalar_type: &ScalarType) -> serde_json::Value;

    /// Same mapping as `default_native_type_for_scalar_type()`, but in the opposite direction.
    fn native_type_is_default_for_scalar_type(&self, native_type: serde_json::Value, scalar_type: &ScalarType) -> bool;

    fn find_native_type_constructor(&self, name: &str) -> Option<&NativeTypeConstructor> {
        self.available_native_type_constructors()
            .iter()
            .find(|constructor| constructor.name.as_str() == name)
    }

    /// This function is used during Schema parsing to calculate the concrete native type.
    /// This powers the use of native types for QE + ME.
    fn parse_native_type(&self, name: &str, args: Vec<String>) -> Result<NativeTypeInstance, ConnectorError>;

    /// This function is used in ME for error messages
    fn render_native_type(&self, native_type: serde_json::Value) -> String {
        let instance = self.introspect_native_type(native_type).unwrap();
        instance.render()
    }

    /// This function is used during introspection to turn an introspected native type into an instance that can be put into the Prisma schema.
    /// powers IE
    fn introspect_native_type(&self, native_type: serde_json::Value) -> Result<NativeTypeInstance, ConnectorError>;

    fn set_config_dir<'a>(&self, config_dir: &std::path::Path, url: &'a str) -> Cow<'a, str> {
        let set_root = |path: &str| {
            let path = std::path::Path::new(path);

            if path.is_relative() {
                Some(config_dir.join(&path).to_str().map(ToString::to_string).unwrap())
            } else {
                None
            }
        };

        let mut url = url::Url::parse(url).unwrap();

        let mut params: BTreeMap<String, String> =
            url.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())).collect();

        url.query_pairs_mut().clear();

        if let Some(path) = params.get("sslcert").map(|s| s.as_str()).and_then(set_root) {
            params.insert("sslcert".into(), path);
        }

        if let Some(path) = params.get("sslidentity").map(|s| s.as_str()).and_then(set_root) {
            params.insert("sslidentity".into(), path);
        }

        for (k, v) in params.into_iter() {
            url.query_pairs_mut().append_pair(&k, &v);
        }

        url.to_string().into()
    }

    fn supports_scalar_lists(&self) -> bool {
        self.has_capability(ConnectorCapability::ScalarLists)
    }

    fn supports_multiple_indexes_with_same_name(&self) -> bool {
        self.has_capability(ConnectorCapability::MultipleIndexesWithSameName)
    }

    fn supports_relations_over_non_unique_criteria(&self) -> bool {
        self.has_capability(ConnectorCapability::RelationsOverNonUniqueCriteria)
    }

    fn supports_enums(&self) -> bool {
        self.has_capability(ConnectorCapability::Enums)
    }

    fn supports_json(&self) -> bool {
        self.has_capability(ConnectorCapability::Json)
    }

    fn supports_auto_increment(&self) -> bool {
        self.has_capability(ConnectorCapability::AutoIncrement)
    }

    fn supports_non_id_auto_increment(&self) -> bool {
        self.has_capability(ConnectorCapability::AutoIncrementAllowedOnNonId)
    }

    fn supports_multiple_auto_increment(&self) -> bool {
        self.has_capability(ConnectorCapability::AutoIncrementMultipleAllowed)
    }

    fn supports_non_indexed_auto_increment(&self) -> bool {
        self.has_capability(ConnectorCapability::AutoIncrementNonIndexedAllowed)
    }

    fn supports_compound_ids(&self) -> bool {
        self.has_capability(ConnectorCapability::CompoundIds)
    }

    fn allows_relation_fields_in_arbitrary_order(&self) -> bool {
        self.has_capability(ConnectorCapability::RelationFieldsInArbitraryOrder)
    }

    fn native_instance_error(&self, instance: NativeTypeInstance) -> ConnectorErrorFactory {
        ConnectorErrorFactory {
            connector: self.name().to_owned(),
            native_type: instance.render(),
        }
    }

    fn native_str_error(&self, native_str: &str) -> ConnectorErrorFactory {
        ConnectorErrorFactory {
            connector: self.name().to_owned(),
            native_type: native_str.to_string(),
        }
    }

    fn native_types_not_supported(&self) -> Result<NativeTypeInstance, ConnectorError> {
        Err(ConnectorError::from_kind(
            ErrorKind::ConnectorNotSupportedForNativeTypes {
                connector_name: self.name().to_owned(),
            },
        ))
    }

    fn validate_url(&self, url: &str) -> Result<(), String>;
}

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

        impl std::fmt::Display for ConnectorCapability {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    RelationsOverNonUniqueCriteria,
    Enums,
    Json,
    AutoIncrement,
    RelationFieldsInArbitraryOrder,
    ForeignKeys,
    //Start of ME/IE only capabilities
    AutoIncrementAllowedOnNonId,
    AutoIncrementMultipleAllowed,
    AutoIncrementNonIndexedAllowed,
    MultipleIndexesWithSameName,
    NamedPrimaryKeys,
    NamedForeignKeys,
    ReferenceCycleDetection,
    // Start of query-engine-only Capabilities
    InsensitiveFilters,
    CreateMany,
    CreateManyWriteableAutoIncId,
    WritableAutoincField,
    CreateSkipDuplicates,
    UpdateableId,
    JsonFiltering,
    JsonFilteringJsonPath,
    JsonFilteringArrayPath,
    CompoundIds,
    AnyId, // Any (or combination of) uniques and not only id fields can constitute an id for a model.
    QueryRaw,
    FullTextSearchWithoutIndex
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
}
