//! Calculate a PSL data model, together with warnings.

mod context;

pub(crate) use context::DatamodelCalculatorContext;

use crate::introspection::{rendering, warnings};
use psl::PreviewFeature;
use schema_connector::{IntrospectionContext, IntrospectionMultiResult, IntrospectionResult};
use sql_schema_describer as sql;

/// Calculate a data model from a database schema.
pub fn calculate(schema: &sql::SqlSchema, ctx: &IntrospectionContext, search_path: &str) -> IntrospectionResult {
    IntrospectionResult::from(calculate_multi(schema, ctx, search_path))
}

/// Calculate datamodels from a database schema.
pub fn calculate_multi(
    schema: &sql::SqlSchema,
    ctx: &IntrospectionContext,
    search_path: &str,
) -> IntrospectionMultiResult {
    let introspection_file_name = ctx.introspection_file_name();
    let ctx = DatamodelCalculatorContext::new(ctx, schema, search_path);

    let (datamodels, is_empty, views) = rendering::to_psl_string(introspection_file_name, &ctx);

    let views = if ctx.config.preview_features().contains(PreviewFeature::Views) {
        Some(views)
    } else {
        None
    };

    let warnings = warnings::generate(&ctx);
    let warnings = match warnings.is_empty() {
        true => None,
        false => Some(warnings.to_string()),
    };

    IntrospectionMultiResult {
        datamodels,
        is_empty,
        warnings,
        views,
    }
}
