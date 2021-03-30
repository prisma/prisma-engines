use super::*;
use constants::inputs::filters;
use output_types::aggregation;

/// Builds "<Model>OrderByInput" object types.
#[tracing::instrument(skip(ctx, model, include_relations))]
pub(crate) fn order_by_object_type(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    include_relations: bool,
    include_scalar_aggregations: bool,
) -> InputObjectTypeWeakRef {
    let enum_type = Arc::new(string_enum_type(
        ordering::SORT_ORDER,
        vec![ordering::ASC.to_owned(), ordering::DESC.to_owned()],
    ));
    let ident_suffix = match (include_relations, include_scalar_aggregations) {
        (false, false) => "",
        (true, false) => "WithRelation",
        (false, true) => "WithAggregation",
        (true, true) => unreachable!("Order by with relations and scalar aggregations is not supported yet"),
    };
    let ident = Identifier::new(format!("{}OrderBy{}Input", model.name, ident_suffix), PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let mut input_object = init_input_object_type(ident.clone());
    input_object.allow_at_most_one_field();

    let input_object = Arc::new(input_object);
    ctx.cache_input_type(ident, input_object.clone());

    let mut fields = model
        .fields()
        .all
        .iter()
        .filter_map(|field| match field {
            ModelField::Relation(rf) if rf.is_list && include_relations => {
                let related_model = rf.related_model();
                let related_object_type = order_by_object_type_rel_aggregate(ctx, &related_model, &enum_type);

                Some(input_field(rf.name.clone(), InputType::object(related_object_type), None).optional())
            }
            ModelField::Relation(rf) if include_relations => {
                let related_model = rf.related_model();
                let related_object_type =
                    order_by_object_type(ctx, &related_model, include_relations, include_scalar_aggregations);

                Some(input_field(rf.name.clone(), InputType::object(related_object_type), None).optional())
            }
            ModelField::Scalar(sf) => {
                Some(input_field(sf.name.clone(), InputType::Enum(enum_type.clone()), None).optional())
            }
            _ => None,
        })
        .collect();

    if include_scalar_aggregations {
        // Fields used in aggregations
        let non_list_nor_json_fields = aggregation::collect_non_list_nor_json_fields(model);
        let numeric_fields = aggregation::collect_numeric_fields(model);

        append_opt(
            &mut fields,
            order_by_field_aggregate(
                filters::UNDERSCORE_COUNT,
                "Count",
                ctx,
                model,
                &enum_type,
                model.fields().scalar(),
            ),
        );
        append_opt(
            &mut fields,
            order_by_field_aggregate(
                filters::UNDERSCORE_AVG,
                "Avg",
                ctx,
                model,
                &enum_type,
                numeric_fields.clone(),
            ),
        );
        append_opt(
            &mut fields,
            order_by_field_aggregate(
                filters::UNDERSCORE_MAX,
                "Max",
                ctx,
                model,
                &enum_type,
                non_list_nor_json_fields.clone(),
            ),
        );
        append_opt(
            &mut fields,
            order_by_field_aggregate(
                filters::UNDERSCORE_MIN,
                "Min",
                ctx,
                model,
                &enum_type,
                non_list_nor_json_fields,
            ),
        );
        append_opt(
            &mut fields,
            order_by_field_aggregate(filters::UNDERSCORE_SUM, "Sum", ctx, model, &enum_type, numeric_fields),
        );
    }

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}

fn order_by_field_aggregate(
    name: &str,
    suffix: &str,
    ctx: &mut BuilderContext,
    model: &ModelRef,
    ordering_enum: &Arc<EnumType>,
    scalar_fields: Vec<ScalarFieldRef>,
) -> Option<InputField> {
    if scalar_fields.is_empty() {
        None
    } else {
        Some(
            input_field(
                name,
                InputType::object(order_by_object_type_aggregate(
                    suffix,
                    ctx,
                    model,
                    ordering_enum,
                    scalar_fields,
                )),
                None,
            )
            .optional(),
        )
    }
}

fn order_by_object_type_aggregate(
    suffix: &str,
    ctx: &mut BuilderContext,
    model: &ModelRef,
    ordering_enum: &Arc<EnumType>,
    scalar_fields: Vec<ScalarFieldRef>,
) -> InputObjectTypeWeakRef {
    let ident = Identifier::new(
        format!("{}{}OrderByAggregateInput", model.name, suffix),
        PRISMA_NAMESPACE,
    );

    return_cached_input!(ctx, &ident);

    let mut input_object = init_input_object_type(ident.clone());
    input_object.require_exactly_one_field();

    let input_object = Arc::new(input_object);
    ctx.cache_input_type(ident, input_object.clone());

    let fields = scalar_fields
        .iter()
        .map(|sf| input_field(sf.name.clone(), InputType::Enum(ordering_enum.clone()), None).optional())
        .collect();

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}

fn order_by_object_type_rel_aggregate(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    ordering_enum: &Arc<EnumType>,
) -> InputObjectTypeWeakRef {
    let ident = Identifier::new(format!("{}OrderByRelationAggregateInput", model.name), PRISMA_NAMESPACE);

    return_cached_input!(ctx, &ident);

    let mut input_object = init_input_object_type(ident.clone());
    input_object.require_exactly_one_field();

    let input_object = Arc::new(input_object);
    ctx.cache_input_type(ident, input_object.clone());

    let fields = vec![input_field(
        filters::COUNT,
        InputType::Enum(ordering_enum.clone()),
        None,
    )];
    input_object.set_fields(fields);

    Arc::downgrade(&input_object)
}
