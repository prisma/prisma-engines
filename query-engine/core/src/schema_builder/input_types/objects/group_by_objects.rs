use super::*;

// pub(crate) fn group_by_input_object_type(ctx: &mut BuilderContext, model: &ModelRef) -> InputObjectTypeWeakRef {
//     let name = format!("{}GroupBySelection", model.name);
//     let ident = Identifier::new(name, PRISMA_NAMESPACE);
//     return_cached_input!(ctx, &ident);

//     let input_object = Arc::new(init_input_object_type(ident.clone()));
//     ctx.cache_input_type(ident, input_object.clone());

//     let fields = vec![
//         input_field("field", InputType::enum_type(model_field_enum(model)), None),
//         input_field("operation", InputType::enum_type(aggregation_operation_enum()), None).optional(),
//     ];

//     input_object.set_fields(fields);
//     Arc::downgrade(&input_object)
// }

pub fn model_field_enum(model: &ModelRef) -> EnumTypeRef {
    Arc::new(EnumType::FieldRef(FieldRefEnumType {
        name: format!("{}GroupByFieldEnum", capitalize(&model.name)),
        values: model
            .fields()
            .scalar()
            .into_iter()
            .map(|field| (field.name.clone(), field))
            .collect(),
    }))
}

// fn aggregation_operation_enum() -> EnumTypeRef {
//     Arc::new(string_enum_type(
//         "AggregateOperationEnum",
//         vec![
//             "count".to_owned(),
//             "avg".to_owned(),
//             "sum".to_owned(),
//             "min".to_owned(),
//             "max".to_owned(),
//         ],
//     ))
// }
