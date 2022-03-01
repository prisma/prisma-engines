use crate::introspection_helpers::*;
use crate::prisma_1_defaults::*;
use crate::re_introspection::enrich;
use crate::sanitize_datamodel_names::{sanitization_leads_to_duplicate_names, sanitize_datamodel_names};
use crate::version_checker::VersionChecker;
use crate::SqlIntrospectionResult;
use crate::{commenting_out_guardrails::commenting_out_guardrails, introspection::introspect};
use datamodel::Datamodel;
use introspection_connector::{IntrospectionContext, IntrospectionResult};
use sql_schema_describer::*;
use tracing::debug;

/// Calculate a data model from a database schema.
pub fn calculate_datamodel(
    schema: &SqlSchema,
    previous_data_model: &Datamodel,
    ctx: IntrospectionContext,
) -> SqlIntrospectionResult<IntrospectionResult> {
    debug!("Calculating data model.");

    let mut version_check = VersionChecker::new(schema, &ctx);
    let mut data_model = Datamodel::new();

    // 1to1 translation of the sql schema
    introspect(schema, &mut version_check, &mut data_model, &ctx)?;

    if !sanitization_leads_to_duplicate_names(&data_model) {
        // our opinionation about valid names
        sanitize_datamodel_names(&mut data_model, &ctx);
    }

    // deduplicating relation field names
    deduplicate_relation_field_names(&mut data_model);

    let mut warnings = vec![];
    if !previous_data_model.is_empty() {
        enrich(previous_data_model, &mut data_model, &ctx, &mut warnings);
        tracing::debug!("Enriching datamodel is done: {:?}", data_model);
    }

    // commenting out models, fields, enums, enum values
    warnings.append(&mut commenting_out_guardrails(&mut data_model, &ctx));

    // try to identify whether the schema was created by a previous Prisma version
    let version = version_check.version(&warnings, &data_model);

    // if based on a previous Prisma version add id default opinionations
    add_prisma_1_id_defaults(&version, &mut data_model, schema, &mut warnings, &ctx);

    // renderer -> parser -> validator, is_commented_out gets lost between renderer and parser
    debug!("Done calculating data model {:?}", data_model);
    Ok(IntrospectionResult {
        data_model,
        warnings,
        version,
    })
}
