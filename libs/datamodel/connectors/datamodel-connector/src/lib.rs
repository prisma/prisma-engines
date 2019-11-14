use crate::scalars::ScalarType;

pub mod error;
pub mod scalars;

mod declarative_connector;
mod example_connector;

pub trait Connector {
    fn calculate_type(&self, name: &str, args: Vec<i32>) -> FieldType;
}

pub struct FieldType {
    name: String,
    prisma_type: scalars::ScalarType,
    datasource_type: String,
}

impl FieldType {
    pub fn prisma_type(&self) -> scalars::ScalarType {
        self.prisma_type
    }

    pub fn datasource_type(&self) -> &str {
        &self.datasource_type
    }
}
