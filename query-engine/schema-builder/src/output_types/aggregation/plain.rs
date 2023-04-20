use super::*;
use crate::constants::aggregations::*;
use std::convert::identity;

/// Builds plain aggregation object type for given model (e.g. AggregateUser).
pub(crate) fn aggregation_object_type(ctx: &mut BuilderContext<'_>, model: &ModelRef) -> OutputObjectTypeId {
    let ident = Identifier::new_prisma(format!("Aggregate{}", capitalize(model.name())));
    return_cached_output!(ctx, &ident);

    let object = ObjectType::new(ident.clone(), Some(model.id));
    let mut object_fields = vec![];

    let non_list_nor_json_fields = collect_non_list_nor_json_fields(&model.into());
    let numeric_fields = collect_numeric_fields(&model.into());

    // Count is available on all fields.
    append_opt(
        &mut object_fields,
        aggregation_field(
            ctx,
            UNDERSCORE_COUNT,
            model,
            model.fields().scalar(),
            |_, _| OutputType::int(),
            |mut obj| {
                obj.add_field(field("_all", vec![], OutputType::int(), None));
                obj
            },
            true,
        ),
    );

    append_opt(
        &mut object_fields,
        aggregation_field(
            ctx,
            UNDERSCORE_AVG,
            model,
            numeric_fields.clone(),
            field_avg_output_type,
            identity,
            false,
        ),
    );

    append_opt(
        &mut object_fields,
        aggregation_field(
            ctx,
            UNDERSCORE_SUM,
            model,
            numeric_fields,
            field::map_scalar_output_type_for_field,
            identity,
            false,
        ),
    );

    append_opt(
        &mut object_fields,
        aggregation_field(
            ctx,
            UNDERSCORE_MIN,
            model,
            non_list_nor_json_fields.clone(),
            field::map_scalar_output_type_for_field,
            identity,
            false,
        ),
    );

    append_opt(
        &mut object_fields,
        aggregation_field(
            ctx,
            UNDERSCORE_MAX,
            model,
            non_list_nor_json_fields,
            field::map_scalar_output_type_for_field,
            identity,
            false,
        ),
    );

    object.set_fields(object_fields);
    ctx.cache_output_type(ident, object)
}
