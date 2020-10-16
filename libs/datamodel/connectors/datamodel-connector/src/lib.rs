mod combined_connector;

pub mod error;
use crate::error::ConnectorError;
pub use combined_connector::CombinedConnector;
use dml::field::Field;
use dml::native_type_constructor::NativeTypeConstructor;
use dml::native_type_instance::NativeTypeInstance;

pub trait Connector: Send + Sync {
    fn capabilities(&self) -> &Vec<ConnectorCapability>;

    fn has_capability(&self, capability: ConnectorCapability) -> bool {
        self.capabilities().contains(&capability)
    }

    fn validate_field(&self, field: &Field) -> Result<(), ConnectorError>;

    /// Returns all available native type constructors available through this connector.
    /// Powers the auto completion of the vs code plugin.
    fn available_native_type_constructors(&self) -> &Vec<NativeTypeConstructor>;

    fn find_native_type_constructor(&self, name: &str) -> Option<&NativeTypeConstructor> {
        self.available_native_type_constructors()
            .iter()
            .find(|constructor| constructor.name.as_str() == name)
    }

    /// This function is used during Schema parsing to calculate the concrete native type.
    /// This powers the use of native types for QE + ME.
    fn parse_native_type(&self, name: &str, args: Vec<u32>) -> Result<NativeTypeInstance, ConnectorError>;

    /// This function is used during introspection to turn an introspected native type into an instance that can be put into the Prisma schema.
    /// powers IE
    fn introspect_native_type(&self, native_type: serde_json::Value) -> Result<NativeTypeInstance, ConnectorError>;

    fn supports_scalar_lists(&self) -> bool {
        self.has_capability(ConnectorCapability::ScalarLists)
    }

    fn supports_multiple_indexes_with_same_name(&self) -> bool {
        self.has_capability(ConnectorCapability::MultipleIndexesWithSameName)
    }

    fn supports_relations_over_non_unique_criteria(&self) -> bool {
        self.has_capability(ConnectorCapability::RelationsOverNonUniqueCriteria)
    }

    fn supports_relations_over_nullable_field(&self) -> bool {
        self.has_capability(ConnectorCapability::RelationsOverNullableField)
    }

    fn supports_enums(&self) -> bool {
        self.has_capability(ConnectorCapability::Enums)
    }

    fn supports_json(&self) -> bool {
        self.has_capability(ConnectorCapability::Json)
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
}

/// Not all Databases are created equal. Hence connectors for our datasources support different capabilities.
/// These are used during schema validation. E.g. if a connector does not support enums an error will be raised.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectorCapability {
    // start of General Schema Capabilities
    ScalarLists,
    RelationsOverNonUniqueCriteria,
    MultipleIndexesWithSameName,
    Enums,
    Json,
    AutoIncrementAllowedOnNonId,
    AutoIncrementMultipleAllowed,
    AutoIncrementNonIndexedAllowed,
    RelationsOverNullableField,
    // start of Query Engine Capabilities
    InsensitiveFilters,
}

/// Contains all capabilities that the connector is able to serve.
#[derive(Debug)]
pub struct ConnectorCapabilities {
    capabilities: Vec<ConnectorCapability>,
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
