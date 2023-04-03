//! Definition of warnings, which are displayed to the user during `db
//! pull`.

mod r#enum;
mod generators;
mod model;
mod view;

use crate::datamodel_calculator::DatamodelCalculatorContext;
use generators::Warnings;
use introspection_connector::Warning;
use psl::PreviewFeature;

/// Analyzes the described database schema, triggering
/// warnings to the user if necessary.
pub(crate) fn generate(ctx: &DatamodelCalculatorContext<'_>) -> Vec<Warning> {
    let mut warnings = Warnings::new();

    for r#enum in ctx.enum_pairs() {
        r#enum::generate_warnings(r#enum, &mut warnings);
    }

    for model in ctx.model_pairs() {
        model::generate_warnings(model, &mut warnings);
    }

    if ctx.config.preview_features().contains(PreviewFeature::Views) {
        for view in ctx.view_pairs() {
            view::generate_warnings(view, &mut warnings);
        }
    }

    warnings.finalize()
}
