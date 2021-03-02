use crate::constants::outputs::fields;

use super::*;
use std::convert::identity;

/// Builds plain aggregation object type for given model (e.g. AggregateUser).
#[tracing::instrument(skip(ctx, model))]
pub(crate) fn aggregation_object_type(ctx: &mut BuilderContext, model: &ModelRef) -> ObjectTypeWeakRef {
    let ident = Identifier::new(format!("Aggregate{}", capitalize(&model.name)), PRISMA_NAMESPACE);
    return_cached_output!(ctx, &ident);

    let object = ObjectTypeStrongRef::new(ObjectType::new(ident.clone(), Some(ModelRef::clone(model))));
    let mut object_fields = vec![];

    let non_list_nor_json_fields = collect_non_list_nor_json_fields(model);
    let numeric_fields = collect_numeric_fields(model);

    // Count is available on all fields.
    append_opt(
        &mut object_fields,
        aggregation_field(
            ctx,
            fields::COUNT,
            &model,
            model.fields().scalar(),
            |_, _| OutputType::int(),
            |mut obj| {
                obj.add_field(field("_all", vec![], OutputType::int(), None));
                obj
            },
        ),
    );

    append_opt(
        &mut object_fields,
        aggregation_field(
            ctx,
            fields::AVG,
            &model,
            numeric_fields.clone(),
            field_avg_output_type,
            identity,
        ),
    );

    append_opt(
        &mut object_fields,
        aggregation_field(
            ctx,
            fields::SUM,
            &model,
            numeric_fields,
            map_scalar_output_type_for_field,
            identity,
        ),
    );

    append_opt(
        &mut object_fields,
        aggregation_field(
            ctx,
            fields::MIN,
            &model,
            non_list_nor_json_fields.clone(),
            map_scalar_output_type_for_field,
            identity,
        ),
    );

    append_opt(
        &mut object_fields,
        aggregation_field(
            ctx,
            fields::MAX,
            &model,
            non_list_nor_json_fields,
            map_scalar_output_type_for_field,
            identity,
        ),
    );

    object.set_fields(object_fields);
    ctx.cache_output_type(ident, ObjectTypeStrongRef::clone(&object));

    ObjectTypeStrongRef::downgrade(&object)
}
