#![deny(rust_2018_idioms, unsafe_code)]

pub mod constraint_names;
pub mod helper;
pub mod walker_ext_traits;

mod empty_connector;
mod referential_integrity;

pub use diagnostics::connector_error;
pub use empty_connector::EmptyDatamodelConnector;
pub use parser_database;
pub use referential_integrity::ReferentialIntegrity;

use crate::connector_error::{ConnectorError, ConnectorErrorFactory, ErrorKind};
use dml::{
    native_type_constructor::NativeTypeConstructor, native_type_instance::NativeTypeInstance,
    relation_info::ReferentialAction, scalars::ScalarType,
};
use enumflags2::BitFlags;
use std::{borrow::Cow, collections::BTreeMap, str::FromStr};

pub trait Connector: Send + Sync {
    fn name(&self) -> &str;

    fn capabilities(&self) -> &'static [ConnectorCapability];

    /// The maximum length of constraint names in bytes. Connectors without a
    /// limit should return usize::MAX.
    fn constraint_name_length(&self) -> usize;

    // Referential integrity

    /// The referential integrity modes that can be set through the referentialIntegrity datasource
    /// argument.
    fn allowed_referential_integrity_settings(&self) -> BitFlags<ReferentialIntegrity> {
        use ReferentialIntegrity::*;

        ForeignKeys | Prisma
    }

    /// The default referential integrity mode to assume for this connector.
    fn default_referential_integrity(&self) -> ReferentialIntegrity {
        ReferentialIntegrity::ForeignKeys
    }

    fn has_capability(&self, capability: ConnectorCapability) -> bool {
        self.capabilities().contains(&capability)
    }

    fn referential_actions(&self, referential_integrity: &ReferentialIntegrity) -> BitFlags<ReferentialAction>;

    fn supports_composite_types(&self) -> bool {
        self.has_capability(ConnectorCapability::CompositeTypes)
    }

    fn supports_named_primary_keys(&self) -> bool {
        self.has_capability(ConnectorCapability::NamedPrimaryKeys)
    }

    fn supports_named_foreign_keys(&self) -> bool {
        self.has_capability(ConnectorCapability::NamedForeignKeys)
    }

    fn supports_named_default_values(&self) -> bool {
        self.has_capability(ConnectorCapability::NamedDefaultValues)
    }

    fn supports_referential_action(&self, integrity: &ReferentialIntegrity, action: ReferentialAction) -> bool {
        self.referential_actions(integrity).contains(action)
    }

    fn validate_field_default_without_native_type(
        &self,
        _field_name: &str,
        _scalar_type: &ScalarType,
        _default: Option<&dml::default_value::DefaultValue>,
        _errors: &mut Vec<ConnectorError>,
    ) {
    }

    /// Validate that the arguments passed to a native type attribute are valid.
    fn validate_native_type_arguments(
        &self,
        _native_type: &NativeTypeInstance,
        _scalar_type: &ScalarType,
        _: &mut Vec<ConnectorError>,
    ) {
    }

    fn validate_model(&self, _model: parser_database::walkers::ModelWalker<'_, '_>, _: &mut Vec<ConnectorError>) {}

    /// The scopes in which a constraint name should be validated. If empty, doesn't check for name
    /// clashes in the validation phase.
    fn constraint_violation_scopes(&self) -> &'static [ConstraintScope] {
        &[]
    }

    /// Returns all available native type constructors available through this connector.
    /// Powers the auto completion of the VSCode plugin.
    fn available_native_type_constructors(&self) -> &'static [NativeTypeConstructor];

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
            .find(|constructor| constructor.name == name)
    }

    /// This function is used during Schema parsing to calculate the concrete native type.
    fn parse_native_type(&self, name: &str, args: Vec<String>) -> Result<NativeTypeInstance, ConnectorError>;

    /// This function is used in ME for error messages
    fn render_native_type(&self, native_type: serde_json::Value) -> String {
        let instance = self.introspect_native_type(native_type).unwrap();
        instance.render()
    }

    /// This function is used during introspection to turn an introspected native type into an instance that can be put into the Prisma schema.
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

        let mut url = match url::Url::parse(url) {
            Ok(url) => url,
            Err(_) => return Cow::from(url), // bail
        };

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

    fn supports_relations_over_non_unique_criteria(&self) -> bool {
        self.has_capability(ConnectorCapability::RelationsOverNonUniqueCriteria)
    }

    fn supports_enums(&self) -> bool {
        self.has_capability(ConnectorCapability::Enums)
    }

    fn supports_json(&self) -> bool {
        self.has_capability(ConnectorCapability::Json)
    }

    fn supports_json_lists(&self) -> bool {
        self.has_capability(ConnectorCapability::JsonLists)
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

    fn native_instance_error(&self, instance: &NativeTypeInstance) -> ConnectorErrorFactory {
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
    EnumArrayPush,
    Json,
    JsonLists,
    AutoIncrement,
    RelationFieldsInArbitraryOrder,
    CompositeTypes,
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
    UsingHashIndex,
    FullTextIndex,
    SortOrderInFullTextIndex,
    MultipleFullTextAttributesPerModel,
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
    JsonFilteringAlphanumeric,
    CompoundIds,
    AnyId, // Any (or combination of) uniques and not only id fields can constitute an id for a model.
    QueryRaw,
    FullTextSearchWithoutIndex,
    AdvancedJsonNullability, // Database distinguishes between their null type and JSON null.
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

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum ConstraintType {
    PrimaryKey,
    ForeignKey,
    KeyOrIdx,
    Default,
}

/// A scope where a constraint name must be unique.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Clone, Copy)]
pub enum ConstraintScope {
    /// Globally indices and unique constraints
    GlobalKeyIndex,
    /// Globally foreign keys
    GlobalForeignKey,
    /// Globally primary keys, indices and unique constraints
    GlobalPrimaryKeyKeyIndex,
    /// Globally primary keys, foreign keys and default constraints
    GlobalPrimaryKeyForeignKeyDefault,
    /// Per model indices and unique constraints
    ModelKeyIndex,
    /// Per model primary keys, indices and unique constraints
    ModelPrimaryKeyKeyIndex,
    /// Per model primary keys, foreign keys, indices and unique constraints
    ModelPrimaryKeyKeyIndexForeignKey,
}

impl ConstraintScope {
    /// A beefed-up display for errors.
    pub fn description(self, model_name: &str) -> Cow<'static, str> {
        match self {
            ConstraintScope::GlobalKeyIndex => Cow::from("global for indexes and unique constraints"),
            ConstraintScope::GlobalForeignKey => Cow::from("global for foreign keys"),
            ConstraintScope::GlobalPrimaryKeyKeyIndex => {
                Cow::from("global for primary key, indexes and unique constraints")
            }
            ConstraintScope::GlobalPrimaryKeyForeignKeyDefault => {
                Cow::from("global for primary keys, foreign keys and default constraints")
            }
            ConstraintScope::ModelKeyIndex => {
                Cow::from(format!("on model `{}` for indexes and unique constraints", model_name))
            }
            ConstraintScope::ModelPrimaryKeyKeyIndex => Cow::from(format!(
                "on model `{}` for primary key, indexes and unique constraints",
                model_name
            )),
            ConstraintScope::ModelPrimaryKeyKeyIndexForeignKey => Cow::from(format!(
                "on model `{}` for primary key, indexes, unique constraints and foreign keys",
                model_name
            )),
        }
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
