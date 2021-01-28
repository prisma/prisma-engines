use super::*;

/// Builds "<Model>OrderByInput" object types.
pub(crate) fn order_by_object_type(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    include_relations: bool,
    seen_relations: &mut Vec<String>,
) -> InputObjectTypeWeakRef {
    let enum_type = Arc::new(string_enum_type(
        ordering::SORT_ORDER,
        vec![ordering::ASC.to_owned(), ordering::DESC.to_owned()],
    ));

    let ident = Identifier::new(format!("{}OrderByInput", model.name), PRISMA_NAMESPACE);
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
            // Only allow to-one relation order-bys.
            ModelField::Relation(rf) if !rf.is_list && !seen_relations.contains(dbg!(&rf.relation().name)) => {
                seen_relations.push(rf.relation().name.clone());

                let related_model = rf.related_model();
                let related_object_type = order_by_object_type(ctx, &related_model, include_relations, seen_relations);

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
