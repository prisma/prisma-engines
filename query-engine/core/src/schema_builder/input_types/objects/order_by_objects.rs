use super::*;
use constants::inputs::filters;

/// Builds "<Model>OrderByInput" object types.
#[tracing::instrument(skip(ctx, model, include_relations))]
pub(crate) fn order_by_object_type(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    include_relations: bool,
) -> InputObjectTypeWeakRef {
    let enum_type = Arc::new(string_enum_type(
        ordering::SORT_ORDER,
        vec![ordering::ASC.to_owned(), ordering::DESC.to_owned()],
    ));

    let with_relation_ident = if include_relations { "WithRelation" } else { "" };
    let ident = Identifier::new(
        format!("{}OrderBy{}Input", model.name, with_relation_ident),
        PRISMA_NAMESPACE,
    );
    return_cached_input!(ctx, &ident);

    let mut input_object = init_input_object_type(ident.clone());
    input_object.allow_at_most_one_field();

    let input_object = Arc::new(input_object);
    ctx.cache_input_type(ident, input_object.clone());

    let fields = model
        .fields()
        .all
        .iter()
        .filter_map(|field| match field {
            ModelField::Relation(rf) if rf.is_list && include_relations => {
                let related_model = rf.related_model();
                let related_object_type = order_by_object_type_aggregate(ctx, &related_model, &enum_type);

                Some(input_field(rf.name.clone(), InputType::object(related_object_type), None).optional())
            }
            ModelField::Relation(rf) if include_relations => {
                let related_model = rf.related_model();
                let related_object_type = order_by_object_type(ctx, &related_model, include_relations);

                Some(input_field(rf.name.clone(), InputType::object(related_object_type), None).optional())
            }
            ModelField::Scalar(sf) => {
                Some(input_field(sf.name.clone(), InputType::Enum(enum_type.clone()), None).optional())
            }
            _ => None,
        })
        .collect();

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}

fn order_by_object_type_aggregate(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    ordering_enum: &Arc<EnumType>,
) -> InputObjectTypeWeakRef {
    let ident = Identifier::new(format!("{}OrderByAggregateInput", model.name), PRISMA_NAMESPACE);

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
