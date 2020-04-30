use crate::scalars::ScalarType;

pub mod error;
pub mod scalars;

mod declarative_connector;
mod example_connector;

pub use declarative_connector::DeclarativeConnector;
pub use example_connector::ExampleConnector;

pub trait Connector {
    fn calculate_type(&self, name: &str, args: Vec<i32>) -> Option<ScalarFieldType>;

    fn supports_scalar_lists(&self) -> bool;

    fn supports_relations_over_non_unique_criteria(&self) -> bool;

    fn supports_enums(&self) -> bool;

    fn supports_json(&self) -> bool;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScalarFieldType {
    name: String,
    prisma_type: scalars::ScalarType,
    datasource_type: String,
}

impl ScalarFieldType {
    pub fn new(name: &str, prisma_type: scalars::ScalarType, datasource_type: &str) -> Self {
        ScalarFieldType {
            name: name.to_string(),
            prisma_type,
            datasource_type: datasource_type.to_string(),
        }
    }

    pub fn prisma_type(&self) -> scalars::ScalarType {
        self.prisma_type
    }

    pub fn datasource_type(&self) -> &str {
        &self.datasource_type
    }
}
