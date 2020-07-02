use crate::commenting_out_guardrails::commenting_out_guardrails;
use crate::introspection::introspect;
use crate::misc_helpers::*;
use crate::prisma_1_defaults::*;
use crate::sanitize_datamodel_names::sanitize_datamodel_names;
use crate::version_checker::VersionChecker;
use crate::SqlIntrospectionResult;
use datamodel::Datamodel;
use introspection_connector::{IntrospectionResult, Warning};
use quaint::connector::SqlFamily;
use sql_schema_describer::*;
use tracing::debug;

/// Calculate a data model from a database schema.
pub fn calculate_datamodel(schema: &SqlSchema, family: &SqlFamily) -> SqlIntrospectionResult<IntrospectionResult> {
    debug!("Calculating data model.");

    let mut version_check = VersionChecker::new(family.clone(), schema);
    let mut data_model = Datamodel::new();

    introspect(schema, &mut version_check, &mut data_model)?;

    sanitize_datamodel_names(&mut data_model);

    let mut warnings: Vec<Warning> = commenting_out_guardrails(&mut data_model);

    deduplicate_field_names(&mut data_model);

    let version = version_check.version(&warnings, &data_model);

    add_prisma_1_id_defaults(family, &version, &mut data_model, schema, &mut warnings);

    debug!("Done calculating data model {:?}", data_model);
    Ok(IntrospectionResult {
        datamodel: data_model,
        version,
        warnings,
    })
}
