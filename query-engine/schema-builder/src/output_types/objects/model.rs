#![allow(clippy::unnecessary_to_owned)]

use super::*;
use crate::constants::aggregations::*;
use std::convert::identity;

/// Compute initial model cache. No fields are computed because we first
/// need all models to be present, then we can compute fields in a second pass.
pub(crate) fn initialize_cache(ctx: &mut BuilderContext) {
    ctx.models().into_iter().for_each(|model| {
        let ident = Identifier::new(&model.name, MODEL_NAMESPACE);
        ctx.cache_output_type(ident.clone(), Arc::new(ObjectType::new(ident, Some(model))));
    });
}

// Compute fields on all cached model object types.
pub(crate) fn initialize_fields(ctx: &mut BuilderContext) {
    ctx.models().into_iter().for_each(|model| {
        let obj: ObjectTypeWeakRef = map_type(ctx, &model);
        let mut fields = compute_model_object_type_fields(ctx, &model);

        // Add _count field. Only include to-many fields.
        let relation_fields = model.fields().relation().into_iter().filter(|f| f.is_list()).collect();

        append_opt(
            &mut fields,
            field::aggregation_relation_field(
                ctx,
                UNDERSCORE_COUNT,
                &model,
                relation_fields,
                |_, _| OutputType::int(),
                identity,
            ),
        );

        obj.into_arc().set_fields(fields);
    });
}

/// Returns an output object type for the given model.
/// Relies on the output type cache being initalized.
pub(crate) fn map_type(ctx: &mut BuilderContext, model: &ModelRef) -> ObjectTypeWeakRef {
    let ident = Identifier::new(model.name.clone(), MODEL_NAMESPACE);
    ctx.get_output_type(&ident)
        .expect("Invariant violation: Initialized output object type for each model.")
}

/// Computes model output type fields.
/// Requires an initialized cache.
fn compute_model_object_type_fields(ctx: &mut BuilderContext, model: &ModelRef) -> Vec<OutputField> {
    model
        .fields()
        .all
        .iter()
        .map(|f| field::map_output_field(ctx, f))
        .collect()
}
