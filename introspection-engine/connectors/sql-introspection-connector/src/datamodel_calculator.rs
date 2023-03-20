//! Calculate a PSL data model, together with warnings.

mod context;

use crate::{rendering, warnings, SqlIntrospectionResult};
pub(crate) use context::DatamodelCalculatorContext;
use introspection_connector::{IntrospectionContext, IntrospectionResult, Version};

use sql_schema_describer as sql;

/// Calculate a data model from a database schema.
pub fn calculate(
    schema: &sql::SqlSchema,
    ctx: &IntrospectionContext,
    search_path: &str,
) -> SqlIntrospectionResult<IntrospectionResult> {
    let ctx = DatamodelCalculatorContext::new(ctx, schema, search_path);

    let (schema_string, is_empty, views) = rendering::to_psl_string(&ctx)?;
    let warnings = warnings::generate(&ctx);

    // Warning codes 5 and 6 are for Prisma 1 default reintrospection.
    let version = if warnings.iter().any(|w| ![5, 6].contains(&w.code)) {
        Version::NonPrisma
    } else {
        ctx.version
    };

    let views = if views.is_empty() { None } else { Some(views) };

    Ok(IntrospectionResult {
        data_model: schema_string,
        is_empty,
        version,
        warnings,
        views,
    })
}
