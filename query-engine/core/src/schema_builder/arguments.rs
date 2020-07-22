use super::*;

/// Builds "where" argument.
pub(crate) fn where_argument(ctx: &mut BuilderContext, model: &ModelRef) -> Argument {
    // let where_object = self
    //     .filter_object_type_builder
    //     .into_arc()
    //     .filter_object_type(Arc::clone(model));

    // argument("where", InputType::opt(InputType::object(where_object)), None)
    todo!()
}

/// Builds "where" argument which input type is the where unique type of the input builder.
pub(crate) fn where_unique_argument(ctx: &mut BuilderContext, model: ModelRef) -> Option<Argument> {
    // let input_object_type = self.input_type_builder.into_arc().where_unique_object_type(&model);

    // if input_object_type.into_arc().is_empty() {
    //     None
    // } else {
    //     Some(argument("where", InputType::object(input_object_type), None))
    // }
    todo!()
}

/// Builds "data" argument intended for the create field.
pub(crate) fn create_arguments(ctx: &mut BuilderContext, model: ModelRef) -> Option<Vec<Argument>> {
    // let input_object_type = self.input_type_builder.into_arc().create_input_type(model, None);

    // if input_object_type.into_arc().is_empty() {
    //     None
    // } else {
    //     Some(vec![argument("data", InputType::object(input_object_type), None)])
    // }
    todo!()
}

/// Builds "where" (unique) argument intended for the delete field.
pub(crate) fn delete_arguments(ctx: &mut BuilderContext, model: ModelRef) -> Option<Vec<Argument>> {
    where_unique_argument(ctx, model).map(|arg| vec![arg])
}

/// Builds "where" (unique) and "data" arguments intended for the update field.
pub(crate) fn update_arguments(ctx: &mut BuilderContext, model: ModelRef) -> Option<Vec<Argument>> {
    where_unique_argument(ctx, Arc::clone(&model)).map(|unique_arg| {
        // let input_object = self.input_type_builder.into_arc().update_input_type(model);
        // let input_object_type = InputType::object(input_object);

        // vec![argument("data", input_object_type, None), unique_arg]
        todo!()
    })
}

/// Builds "where" (unique), "create", and "update" arguments intended for the upsert field.
pub(crate) fn upsert_arguments(ctx: &mut BuilderContext, model: ModelRef) -> Option<Vec<Argument>> {
    // self.where_unique_argument(Arc::clone(&model))
    //     .and_then(|where_unique_arg| {
    //         let update_type = self.input_type_builder.into_arc().update_input_type(Arc::clone(&model));
    //         let create_type = self
    //             .input_type_builder
    //             .into_arc()
    //             .create_input_type(Arc::clone(&model), None);

    //         if update_type.into_arc().is_empty() || create_type.into_arc().is_empty() {
    //             None
    //         } else {
    //             Some(vec![
    //                 where_unique_arg,
    //                 argument("create", InputType::object(create_type), None),
    //                 argument("update", InputType::object(update_type), None),
    //             ])
    //         }
    //     })
    todo!()
}

/// Builds "where" and "data" arguments intended for the update many field.
pub(crate) fn update_many_arguments(ctx: &mut BuilderContext, model: ModelRef) -> Vec<Argument> {
    // let update_object = self
    //     .input_type_builder
    //     .into_arc()
    //     .update_many_input_type(Arc::clone(&model));

    // let where_arg = self.where_argument(&model);

    // vec![argument("data", InputType::object(update_object), None), where_arg]
    todo!()
}

/// Builds "where" argument intended for the delete many field.
pub(crate) fn delete_many_arguments(ctx: &mut BuilderContext, model: ModelRef) -> Vec<Argument> {
    // let where_arg = self.where_argument(&model);

    // vec![where_arg]
    todo!()
}

/// Builds "many records where" arguments based on the given model and field.
pub(crate) fn many_records_field_arguments(ctx: &mut BuilderContext, field: &ModelField) -> Vec<Argument> {
    // match field {
    //     ModelField::Scalar(_) => vec![],
    //     ModelField::Relation(rf) if rf.is_list && !rf.related_model().is_embedded => {
    //         self.many_records_arguments(&rf.related_model())
    //     }
    //     ModelField::Relation(rf) if rf.is_list && rf.related_model().is_embedded => vec![],
    //     ModelField::Relation(rf) if !rf.is_list => vec![],
    //     _ => unreachable!(),
    // }
    todo!()
}

/// Builds "many records where" arguments solely based on the given model.
pub(crate) fn many_records_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Vec<Argument> {
    // let unique_input_type = InputType::opt(InputType::object(
    //     self.input_type_builder.into_arc().where_unique_object_type(model),
    // ));

    // let mut args = vec![
    //     self.where_argument(&model),
    //     self.order_by_argument(&model),
    //     argument("cursor", unique_input_type.clone(), None),
    //     argument("take", InputType::opt(InputType::int()), None),
    //     argument("skip", InputType::opt(InputType::int()), None),
    // ];

    // if feature_flags::get().distinct {
    //     let enum_type = Arc::new(EnumType::FieldRef(FieldRefEnumType {
    //         name: format!("{}DistinctFieldEnum", capitalize(&model.name)),
    //         values: model
    //             .fields()
    //             .scalar()
    //             .into_iter()
    //             .map(|field| (field.name.clone(), field))
    //             .collect(),
    //     }));

    //     args.push(argument(
    //         "distinct",
    //         InputType::opt(InputType::list(InputType::Enum(enum_type))),
    //         None,
    //     ));
    // }

    // args
    todo!()
}

// Builds "orderBy" argument.
pub(crate) fn order_by_argument(ctx: &mut BuilderContext, model: &ModelRef) -> Argument {
    // enum_type.into()
    // todo object type

    // let enum_values: Vec<_> = model
    //     .fields()
    //     .scalar()
    //     .into_iter()
    //     .filter(|field| !field.is_list)
    //     .map(|field| {
    //         vec![
    //             (
    //                 format!("{}_{}", field.name, SortOrder::Ascending.to_string()),
    //                 OrderBy {
    //                     field: field.clone(),
    //                     sort_order: SortOrder::Ascending,
    //                 },
    //             ),
    //             (
    //                 format!("{}_{}", field.name, SortOrder::Descending.to_string()),
    //                 OrderBy {
    //                     field: field.clone(),
    //                     sort_order: SortOrder::Descending,
    //                 },
    //             ),
    //         ]
    //     })
    //     .flatten()
    //     .collect();

    // let object_type = self.order_by_object_type(model);

    // argument("orderBy", InputType::opt(InputType::object(object_type)), None)
    todo!()
}
