//! Definition of warnings, which are displayed to the user during `db
//! pull`.

mod r#enum;
mod model;
mod view;

use crate::introspection::datamodel_calculator::DatamodelCalculatorContext;
use psl::PreviewFeature;
use schema_connector::{warnings::Model, Warnings};

use super::introspection_pair::RelationFieldDirection;

/// Analyzes the described database schema, triggering
/// warnings to the user if necessary.
pub(crate) fn generate(ctx: &DatamodelCalculatorContext<'_>) -> Warnings {
    let mut warnings = Warnings::new();

    for r#enum in ctx.enum_pairs() {
        r#enum::generate_warnings(r#enum, &mut warnings);
    }

    for model in ctx.model_pairs() {
        model::generate_warnings(model, &mut warnings);
    }

    for (dir, fk) in ctx.m2m_relations() {
        let Some(model) = ctx.existing_model(fk.referenced_table().id) else {
            continue;
        };
        let Some(rel) = ctx.existing_m2m_relation(fk.table().id) else {
            continue;
        };
        let expected_model = match dir {
            RelationFieldDirection::Back => rel.model_a(),
            RelationFieldDirection::Forward => rel.model_b(),
        };
        if model != expected_model {
            let pair = if rel.model_a().id < rel.model_b().id {
                (Model::new(rel.model_a().name()), Model::new(rel.model_b().name()))
            } else {
                (Model::new(rel.model_b().name()), Model::new(rel.model_a().name()))
            };
            warnings.broken_m2m_relations.insert(pair);
        }
    }

    if ctx.config.preview_features().contains(PreviewFeature::Views) {
        for view in ctx.view_pairs() {
            view::generate_warnings(view, &mut warnings);
        }
    }

    ctx.flavour.generate_warnings(ctx, &mut warnings);

    warnings
}
