//! This module contains the models representing the Datamodel part of a Prisma schema.
//! It contains the main data structures that the engines can build upon.
mod datamodel;
mod default_value;
mod r#enum;
mod field;
mod model;
mod relation_info;
mod traits;

pub use self::datamodel::*;
pub use default_value::*;
pub use field::*;
pub use model::*;
pub use r#enum::*;
pub use relation_info::*;
pub use traits::*;

// Compatibility exports so that users of this module don't need to import the connector as well.
pub use datamodel_connector::scalars::ScalarType;
