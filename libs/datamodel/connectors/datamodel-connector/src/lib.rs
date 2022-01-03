#![deny(rust_2018_idioms, unsafe_code)]

pub mod capabilities;
pub mod constraint_names;
pub mod helper;
pub mod walker_ext_traits;

mod empty_connector;
mod native_type_constructor;
mod referential_integrity;

pub use self::capabilities::{ConnectorCapabilities, ConnectorCapability};
pub use diagnostics::{connector_error, DatamodelError, Diagnostics};
pub use empty_connector::EmptyDatamodelConnector;
pub use native_type_constructor::NativeTypeConstructor;
pub use parser_database::{self, ReferentialAction, ScalarType};
pub use referential_integrity::ReferentialIntegrity;

use crate::connector_error::{ConnectorError, ConnectorErrorFactory, ErrorKind};
use dml::native_type_instance::NativeTypeInstance;
use enumflags2::BitFlags;
use std::{borrow::Cow, collections::BTreeMap};

pub trait Connector: Send + Sync {
    /// The name of the connector. Can be used in error messages.
    fn name(&self) -> &str;

    // Capabilities

    /// The static list of capabilities for the connector.
    fn capabilities(&self) -> &'static [ConnectorCapability];

    fn has_capability(&self, capability: ConnectorCapability) -> bool {
        self.capabilities().contains(&capability)
    }

    /// The maximum length of constraint names in bytes. Connectors without a
    /// limit should return usize::MAX.
    fn max_identifier_length(&self) -> usize;

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

    /// Validate that the arguments passed to a native type attribute are valid.
    fn validate_native_type_arguments(
        &self,
        _native_type: &NativeTypeInstance,
        _scalar_type: &ScalarType,
        _: &mut Vec<ConnectorError>,
    ) {
    }

    fn validate_model(&self, _model: parser_database::walkers::ModelWalker<'_, '_>, _: &mut diagnostics::Diagnostics) {}

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

/// (temporary) bridge between dml::scalars::ScalarType and parser_database::ScalarType. Avoid
/// relying on this if you can.
pub fn convert_from_scalar_type(st: dml::scalars::ScalarType) -> ScalarType {
    match st {
        dml::scalars::ScalarType::Int => ScalarType::Int,
        dml::scalars::ScalarType::BigInt => ScalarType::BigInt,
        dml::scalars::ScalarType::Float => ScalarType::Float,
        dml::scalars::ScalarType::Boolean => ScalarType::Boolean,
        dml::scalars::ScalarType::String => ScalarType::String,
        dml::scalars::ScalarType::DateTime => ScalarType::DateTime,
        dml::scalars::ScalarType::Json => ScalarType::Json,
        dml::scalars::ScalarType::Bytes => ScalarType::Bytes,
        dml::scalars::ScalarType::Decimal => ScalarType::Decimal,
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
