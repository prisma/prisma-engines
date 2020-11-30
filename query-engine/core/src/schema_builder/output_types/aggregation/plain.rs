use super::*;
use std::convert::identity;

/// Builds plain aggregation object type for given model (e.g. AggregateUser).
pub(crate) fn aggregation_object_type(ctx: &mut BuilderContext, model: &ModelRef) -> ObjectTypeWeakRef {
    let ident = Identifier::new(format!("Aggregate{}", capitalize(&model.name)), PRISMA_NAMESPACE);
    return_cached_output!(ctx, &ident);

    let object = ObjectTypeStrongRef::new(ObjectType::new(ident.clone(), Some(ModelRef::clone(model))));
    let mut object_fields = vec![];

    let non_list_fields = collect_non_list_fields(model);
    let numeric_fields = collect_numeric_fields(model);

    // Count is available on all fields.
    append_opt(
        &mut object_fields,
        aggregation_field(
            ctx,
            "count",
            &model,
            model.fields().scalar(),
            |_| OutputType::int(),
            |obj| obj.do_allow_empty(),
        ),
    );

    append_opt(
        &mut object_fields,
        aggregation_field(
            ctx,
            "avg",
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
            "sum",
            &model,
            numeric_fields.clone(),
            map_scalar_output_type,
            identity,
        ),
    );

    append_opt(
        &mut object_fields,
        aggregation_field(
            ctx,
            "min",
            &model,
            non_list_fields.clone(),
            map_scalar_output_type,
            identity,
        ),
    );

    append_opt(
        &mut object_fields,
        aggregation_field(ctx, "max", &model, non_list_fields, map_scalar_output_type, identity),
    );

    object.set_fields(object_fields);
    ctx.cache_output_type(ident, ObjectTypeStrongRef::clone(&object));

    ObjectTypeStrongRef::downgrade(&object)
}
