use crate::{
    commenting_out_guardrails::commenting_out_guardrails,
    introspection::introspect,
    introspection_helpers::*,
    prisma_1_defaults::*,
    re_introspection::enrich,
    sanitize_datamodel_names::{sanitization_leads_to_duplicate_names, sanitize_datamodel_names},
    version_checker, SqlFamilyTrait, SqlIntrospectionResult,
};
use enumflags2::BitFlags;
use introspection_connector::{IntrospectionContext, IntrospectionResult};
use psl::{builtin_connectors::*, common::preview_features::PreviewFeature, dml::Datamodel, Datasource};
use quaint::prelude::SqlFamily;
use sql_schema_describer::SqlSchema;
use tracing::debug;

pub(crate) struct CalculateDatamodelContext<'a> {
    pub source: &'a Datasource,
    pub preview_features: BitFlags<PreviewFeature>,
    pub datamodel: &'a mut Datamodel,
    pub schema: &'a SqlSchema,
    pub sql_family: SqlFamily,
}

impl CalculateDatamodelContext<'_> {
    pub(crate) fn is_cockroach(&self) -> bool {
        self.source.active_connector.provider_name() == COCKROACH.provider_name()
    }
}

/// Calculate a data model from a database schema.
pub fn calculate_datamodel(
    schema: &SqlSchema,
    ctx: &IntrospectionContext,
) -> SqlIntrospectionResult<IntrospectionResult> {
    debug!("Calculating data model.");

    let previous_datamodel = &ctx.previous_data_model;
    let mut datamodel = Datamodel::new();

    let mut context = CalculateDatamodelContext {
        source: &ctx.source,
        preview_features: ctx.preview_features,
        datamodel: &mut datamodel,
        schema,
        sql_family: ctx.sql_family(),
    };

    // 1to1 translation of the sql schema
    introspect(&mut context)?;

    if !sanitization_leads_to_duplicate_names(context.datamodel) {
        // our opinionation about valid names
        sanitize_datamodel_names(&mut context);
    }

    // deduplicating relation field names
    deduplicate_relation_field_names(&mut datamodel);

    let mut warnings = vec![];
    if !previous_datamodel.is_empty() {
        enrich(previous_datamodel, &mut datamodel, ctx, &mut warnings);
        debug!("Enriching datamodel is done.");
    }

    // commenting out models, fields, enums, enum values
    warnings.append(&mut commenting_out_guardrails(&mut datamodel, ctx));

    // try to identify whether the schema was created by a previous Prisma version
    let version = version_checker::check_prisma_version(schema, ctx, &mut warnings);

    // if based on a previous Prisma version add id default opinionations
    add_prisma_1_id_defaults(&version, &mut datamodel, schema, &mut warnings, ctx);

    debug!("Done calculating datamodel.");
    Ok(IntrospectionResult {
        data_model: datamodel,
        warnings,
        version,
    })
}
