//! Calculate a PSL data model, together with warnings.

mod context;

pub(crate) use context::DatamodelCalculatorContext;

use crate::introspection::{rendering, warnings};
use psl::PreviewFeature;
use schema_connector::{IntrospectionContext, IntrospectionResult};
use sql_schema_describer as sql;

/// Calculate a data model from a database schema.
pub fn calculate(schema: &sql::SqlSchema, ctx: &IntrospectionContext, search_path: &str) -> IntrospectionResult {
    let ctx = DatamodelCalculatorContext::new(ctx, schema, search_path);

    let (schema_string, is_empty, views) = rendering::to_psl_string(&ctx);
    let warnings = warnings::generate(&ctx);

    let empty_warnings = warnings.is_empty();

    let views = if ctx.config.preview_features().contains(PreviewFeature::Views) {
        Some(views)
    } else {
        None
    };

    let warnings = if empty_warnings {
        None
    } else {
        Some(warnings.to_string())
    };

    IntrospectionResult {
        data_model: schema_string,
        is_empty,
        warnings,
        views,
    }
}
