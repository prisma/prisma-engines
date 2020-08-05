mod combined_connector;
mod declarative_connector;

pub mod error;
pub mod scalars;

pub use combined_connector::CombinedConnector;
pub use declarative_connector::{DeclarativeConnector, FieldTypeConstructor};
use native_types::NativeType;
use serde::de::DeserializeOwned;

pub trait Connector: Send + Sync {
    fn capabilities(&self) -> &Vec<ConnectorCapability>;

    fn has_capability(&self, capability: ConnectorCapability) -> bool {
        self.capabilities().contains(&capability)
    }

    // TODO carmen: This should return a Result<ScalarFieldType, ConnectorError> instead.
    // possible errors: unknown type name, wrong number of arguments, declared field type is not compatible with native type
    fn calculate_native_type(&self, name: &str, args: Vec<u32>) -> Option<ScalarFieldType>;

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
}

/// Not all Databases are created equal. Hence connectors for our datasources support different capabilities.
/// These are used during schema validation. E.g. if a connector does not support enums an error will be raised.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectorCapability {
    ScalarLists,
    RelationsOverNonUniqueCriteria,
    MultipleIndexesWithSameName,
    Enums,
    Json,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScalarFieldType {
    name: String,
    prisma_type: scalars::ScalarType,
    serialized_native_type: serde_json::Value,
}

impl ScalarFieldType {
    pub fn new(name: &str, prisma_type: scalars::ScalarType, native_type: &dyn NativeType) -> Self {
        ScalarFieldType {
            name: name.to_string(),
            prisma_type,
            serialized_native_type: native_type.to_json(),
        }
    }

    pub fn prisma_type(&self) -> scalars::ScalarType {
        self.prisma_type
    }

    pub fn native_type<T>(&self) -> T
    where
        T: DeserializeOwned,
    {
        let error_msg = format!(
            "Deserializing the native type from json failed: {:?}",
            self.serialized_native_type.as_str()
        );
        serde_json::from_value(self.serialized_native_type.clone()).expect(&error_msg)
    }
}
