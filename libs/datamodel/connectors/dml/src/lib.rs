//! This module contains the models representing the Datamodel part of a Prisma schema.
//! It contains the main data structures that the engines can build upon.

pub mod datamodel;
pub mod default_value;
pub mod r#enum;
pub mod field;
pub mod model;
pub mod relation_info;
pub mod traits;

// Compatibility exports so that users of this module don't need to import the connector as well.
use crate::relation_info::RelationInfo;
pub use datamodel_connector::scalars::ScalarType;
