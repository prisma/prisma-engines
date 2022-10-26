//! This module contains the models representing the Datamodel part of a Prisma schema.
//! It contains the main data structures that the engines can build upon.

#![allow(clippy::derive_partial_eq_without_eq)]

mod datamodel;
mod lift;

pub mod composite_type;
pub mod default_value;
pub mod r#enum;
pub mod field;
pub mod model;
pub mod native_type_instance;
pub mod relation_info;
pub mod scalars;
pub mod traits;

pub use self::{
    composite_type::*, datamodel::*, default_value::*, field::*, model::*, native_type_instance::*, r#enum::*,
    relation_info::*, scalars::*, traits::*,
};
pub use prisma_value::{self, PrismaValue};

use psl_core::ValidatedSchema;

/// Find the model mapping to the passed in database name.
pub fn find_model_by_db_name<'a>(datamodel: &'a Datamodel, db_name: &str) -> Option<&'a Model> {
    datamodel
        .models
        .iter()
        .find(|model| model.database_name() == Some(db_name) || model.name == db_name)
}

/// Validated schema -> dml::Datamodel.
pub fn lift(schema: &ValidatedSchema) -> crate::Datamodel {
    lift::LiftAstToDml::new(&schema.db, schema.connector, schema.relation_mode()).lift()
}
