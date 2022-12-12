//! Calculate a PSL data model, together with warnings.

mod context;

use crate::{rendering, warnings, SqlFamilyTrait, SqlIntrospectionResult};
pub(crate) use context::{InputContext, OutputContext};
use introspection_connector::{IntrospectionContext, IntrospectionResult, Version};

use sql_schema_describer as sql;

/// Calculate a data model from a database schema.
pub fn calculate(schema: &sql::SqlSchema, ctx: &IntrospectionContext) -> SqlIntrospectionResult<IntrospectionResult> {
    let introspection_map = Default::default();

    let mut input = InputContext {
        version: Version::NonPrisma,
        config: ctx.configuration(),
        render_config: ctx.render_config,
        schema,
        sql_family: ctx.sql_family(),
        previous_schema: ctx.previous_schema(),
        introspection_map: &introspection_map,
    };

    let introspection_map = crate::introspection_map::IntrospectionMap::new(input);
    input.introspection_map = &introspection_map;

    let mut output = OutputContext {
        rendered_schema: datamodel_renderer::Datamodel::default(),
        warnings: warnings::Warnings::new(),
    };

    input.version = crate::version_checker::check_prisma_version(&input);

    let (schema_string, is_empty) = rendering::to_psl_string(input, &mut output)?;
    let warnings = output.finalize_warnings();

    // Warning codes 5 and 6 are for Prisma 1 default reintrospection.
    let version = if warnings.iter().any(|w| ![5, 6].contains(&w.code)) {
        Version::NonPrisma
    } else {
        input.version
    };

    Ok(IntrospectionResult {
        data_model: schema_string,
        is_empty,
        version,
        warnings,
    })
}
