//! This module contains the models representing the Datamodel part of a Prisma schema.
//! It contains the main data structures that the engines can build upon.

#![allow(clippy::derive_partial_eq_without_eq)]

mod datamodel;
mod lift;
mod render;

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

use psl_core::{reformat, Configuration, ValidatedSchema};

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

/// Renders the datamodel _without configuration blocks_.
pub fn render_datamodel_to_string(datamodel: &crate::Datamodel, configuration: Option<&Configuration>) -> String {
    let datasource = configuration.and_then(|c| c.datasources.first());
    let mut out = String::new();
    render::render_datamodel(render::RenderParams { datasource, datamodel }, &mut out);
    reformat(&out, DEFAULT_INDENT_WIDTH).expect("Internal error: failed to reformat introspected schema")
}

/// Renders a datamodel, sources and generators.
pub fn render_datamodel_and_config_to_string(datamodel: &crate::Datamodel, config: &Configuration) -> String {
    let mut out = String::new();
    let datasource = config.datasources.first();
    render::render_configuration(config, &mut out);
    render::render_datamodel(render::RenderParams { datasource, datamodel }, &mut out);
    reformat(&out, DEFAULT_INDENT_WIDTH).expect("Internal error: failed to reformat introspected schema")
}

const DEFAULT_INDENT_WIDTH: usize = 2;
