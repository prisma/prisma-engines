use crate::introspection_helpers::*;
use crate::prisma_1_defaults::*;
use crate::re_introspection::enrich;
use crate::sanitize_datamodel_names::{sanitization_leads_to_duplicate_names, sanitize_datamodel_names};
use crate::version_checker::VersionChecker;
use crate::SqlIntrospectionResult;
use crate::{commenting_out_guardrails::commenting_out_guardrails, introspection::introspect};
use datamodel::Datamodel;
use introspection_connector::IntrospectionContext;
use introspection_connector::{IntrospectionResult, IntrospectionSettings};
use sql_schema_describer::*;
use tracing::debug;

/// Calculate a data model from a database schema.
pub fn calculate_datamodel(
    schema: &SqlSchema,
    context: &IntrospectionContext<'_>,
    settings: IntrospectionSettings,
) -> SqlIntrospectionResult<IntrospectionResult> {
    debug!("Calculating data model.");

    let mut version_check = VersionChecker::new(schema, &settings);
    let mut data_model = Datamodel::new();

    // 1to1 translation of the sql schema
    introspect(schema, &mut version_check, &mut data_model, &settings)?;

    if !sanitization_leads_to_duplicate_names(&data_model) {
        // our opinionation about valid names
        sanitize_datamodel_names(&mut data_model, &settings);
    }

    // deduplicating relation field names
    deduplicate_relation_field_names(&mut data_model);

    let mut warnings = vec![];
    if context.has_existing_data_model() {
        enrich(context, &mut data_model, &settings, &mut warnings);
        tracing::debug!("Enriching datamodel is done: {:?}", data_model);
    }

    // commenting out models, fields, enums, enum values
    warnings.append(&mut commenting_out_guardrails(&mut data_model, &settings));

    // try to identify whether the schema was created by a previous Prisma version
    let version = version_check.version(&warnings, &data_model);

    // if based on a previous Prisma version add id default opinionations
    add_prisma_1_id_defaults(&version, &mut data_model, schema, &mut warnings, &settings);

    // renderer -> parser -> validator, is_commented_out gets lost between renderer and parser
    debug!("Done calculating data model {:?}", data_model);
    Ok(IntrospectionResult {
        data_model,
        warnings,
        version,
    })
}
