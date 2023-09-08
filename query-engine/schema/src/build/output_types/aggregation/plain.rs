use super::*;
use constants::aggregations::*;
use std::convert::identity;

/// Builds plain aggregation object type for given model (e.g. AggregateUser).
pub(crate) fn aggregation_object_type(ctx: &'_ QuerySchema, model: Model) -> ObjectType<'_> {
    let ident = Identifier::new_prisma(format!("Aggregate{}", capitalize(model.name())));
    let model_id = model.id;
    let mut obj = ObjectType::new(ident, move || {
        let mut object_fields = vec![];

        let non_list_nor_json_fields = collect_non_list_nor_json_fields(&model.clone().into());
        let numeric_fields = collect_numeric_fields(&model.clone().into());

        // Count is available on all fields.
        append_opt(
            &mut object_fields,
            aggregation_field(
                ctx,
                UNDERSCORE_COUNT,
                &model,
                model.fields().scalar().collect(),
                |_, _| OutputType::non_list(OutputType::int()),
                |mut obj| {
                    obj.fields = Arc::new(once_cell::sync::Lazy::new(Box::new(move || {
                        let mut fields: Vec<_> = (*obj.fields.as_ref()).clone();
                        fields.push(field_no_arguments(
                            "_all",
                            OutputType::non_list(OutputType::int()),
                            None,
                        ));
                        fields
                    })));
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
                &model,
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
                &model,
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
                &model,
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
                &model,
                non_list_nor_json_fields,
                field::map_scalar_output_type_for_field,
                identity,
                false,
            ),
        );

        object_fields
    });

    obj.model = Some(model_id);
    obj
}
