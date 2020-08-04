//! This module contains the models representing the Datasources and Generators of a Prisma schema.
mod configuration;
mod datasource;
mod generator;

pub use configuration::*;
pub use datasource::*;
pub use generator::*;
