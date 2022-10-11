use crate::{introspection::introspect, SqlFamilyTrait, SqlIntrospectionResult};
use introspection_connector::{IntrospectionContext, IntrospectionResult};
use psl::{builtin_connectors::*, datamodel_connector::Connector, dml::Datamodel, Configuration};
use quaint::prelude::SqlFamily;
use sql_schema_describer::SqlSchema;
use tracing::debug;

pub(crate) struct CalculateDatamodelContext<'a> {
    pub config: &'a Configuration,
    pub render_config: bool,
    pub previous_datamodel: &'a Datamodel,
    pub schema: &'a SqlSchema,
    pub sql_family: SqlFamily,
}

impl CalculateDatamodelContext<'_> {
    pub(crate) fn is_cockroach(&self) -> bool {
        self.active_connector().provider_name() == COCKROACH.provider_name()
    }

    pub(crate) fn foreign_keys_enabled(&self) -> bool {
        self.config
            .datasources
            .first()
            .unwrap()
            .relation_mode()
            .uses_foreign_keys()
    }

    pub(crate) fn active_connector(&self) -> &dyn Connector {
        self.config.datasources.first().unwrap().active_connector
    }
}

/// Calculate a data model from a database schema.
pub fn calculate_datamodel(
    schema: &SqlSchema,
    ctx: &IntrospectionContext,
) -> SqlIntrospectionResult<IntrospectionResult> {
    debug!("Calculating data model.");

    let previous_datamodel = &ctx.previous_data_model;
    let context = CalculateDatamodelContext {
        config: ctx.configuration(),
        render_config: ctx.render_config,
        previous_datamodel,
        schema,
        sql_family: ctx.sql_family(),
    };

    let mut warnings = Vec::new();

    let (version, data_model, is_empty) = introspect(&context, &mut warnings)?;

    debug!("Done calculating datamodel.");

    Ok(IntrospectionResult {
        data_model,
        is_empty,
        warnings,
        version,
    })
}
