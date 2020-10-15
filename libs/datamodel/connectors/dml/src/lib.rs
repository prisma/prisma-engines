//! This module contains the models representing the Datamodel part of a Prisma schema.
//! It contains the main data structures that the engines can build upon.

pub mod datamodel;
pub mod default_value;
pub mod r#enum;
pub mod field;
pub mod model;
pub mod native_type_constructor;
pub mod native_type_instance;
pub mod relation_info;
pub mod scalars;
pub mod traits;

use crate::relation_info::RelationInfo;
