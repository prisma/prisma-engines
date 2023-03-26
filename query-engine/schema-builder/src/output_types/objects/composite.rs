#![allow(clippy::unnecessary_to_owned)]

use super::*;
use prisma_models::CompositeType;

/// Compute initial composites cache. No fields are computed because we first
/// need all composites to be present, then we can compute fields in a second pass.
pub(crate) fn initialize_cache(ctx: &mut BuilderContext<'_>) {
    for composite in ctx.internal_data_model.composite_types() {
        let ident = Identifier::new_model(composite.name());
        ctx.cache_output_type(ident.clone(), Arc::new(ObjectType::new(ident, None)));
    }
}

// Compute fields on all cached composite object types.
pub(crate) fn initialize_fields(ctx: &mut BuilderContext<'_>) {
    for composite in ctx.internal_data_model.composite_types() {
        let fields = compute_composite_object_type_fields(ctx, &composite);
        let obj: ObjectTypeWeakRef = map_type(ctx, &composite);

        obj.into_arc().set_fields(fields);
    }
}

pub(crate) fn map_type(ctx: &mut BuilderContext<'_>, ct: &CompositeType) -> ObjectTypeWeakRef {
    let ident = Identifier::new_model(ct.name());
    ctx.get_output_type(&ident)
        .expect("Invariant violation: Initialized output object type for each composite.")
}

/// Computes composite output type fields.
/// Requires an initialized cache.
fn compute_composite_object_type_fields(ctx: &mut BuilderContext<'_>, composite: &CompositeType) -> Vec<OutputField> {
    composite.fields().map(|f| field::map_output_field(ctx, &f)).collect()
}
