#![allow(clippy::unnecessary_to_owned)]

use super::*;
use query_structure::CompositeType;

pub(crate) fn composite_object_type(ctx: &'_ QuerySchema, composite: CompositeType) -> ObjectType<'_> {
    ObjectType::new(Identifier::new_model(composite.name().to_owned()), move || {
        compute_composite_object_type_fields(ctx, &composite)
    })
}

/// Computes composite output type fields.
fn compute_composite_object_type_fields<'a>(ctx: &'a QuerySchema, composite: &CompositeType) -> Vec<OutputField<'a>> {
    composite.fields().map(|f| field::map_output_field(ctx, f)).collect()
}
