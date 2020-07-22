use super::*;

pub(crate) fn build(ctx: &mut BuilderContext) -> (OutputType, ObjectTypeStrongRef) {
    // let non_embedded_models = self.non_embedded_models();
    //     let mut fields: Vec<Field> = non_embedded_models
    //         .into_iter()
    //         .map(|model| {
    //             let mut vec = vec![self.create_item_field(Arc::clone(&model))];

    //             append_opt(&mut vec, self.delete_item_field(Arc::clone(&model)));
    //             append_opt(&mut vec, self.update_item_field(Arc::clone(&model)));
    //             append_opt(&mut vec, self.upsert_item_field(Arc::clone(&model)));

    //             vec.push(self.update_many_field(Arc::clone(&model)));
    //             vec.push(self.delete_many_field(Arc::clone(&model)));

    //             vec
    //         })
    //         .flatten()
    //         .collect();

    //     if self.enable_raw_queries {
    //         fields.push(self.create_execute_raw_field());
    //         fields.push(self.create_query_raw_field());
    //     }

    //     let strong_ref = Arc::new(object_type("Mutation", fields, None));

    //     (OutputType::Object(Arc::downgrade(&strong_ref)), strong_ref)
    todo!()
}

fn create_execute_raw_field(ctx: &mut BuilderContext) -> Field {
    // field(
    //     "executeRaw",
    //     vec![
    //         argument("query", InputType::string(), None),
    //         argument(
    //             "parameters",
    //             InputType::opt(InputType::json_list()),
    //             Some(dml::DefaultValue::Single(PrismaValue::String("[]".into()))),
    //         ),
    //     ],
    //     OutputType::json(),
    //     None,
    // )
    todo!()
}

fn create_query_raw_field(ctx: &mut BuilderContext) -> Field {
    // field(
    //     "queryRaw",
    //     vec![
    //         argument("query", InputType::string(), None),
    //         argument(
    //             "parameters",
    //             InputType::opt(InputType::json_list()),
    //             Some(dml::DefaultValue::Single(PrismaValue::String("[]".into()))),
    //         ),
    //     ],
    //     OutputType::json(),
    //     None,
    // )
    todo!()
}

/// Builds a create mutation field (e.g. createUser) for given model.
fn create_item_field(ctx: &mut BuilderContext, model: ModelRef) -> Field {
    // let args = self
    //     .argument_builder
    //     .create_arguments(Arc::clone(&model))
    //     .unwrap_or_else(|| vec![]);

    // let field_name = self.pluralize_internal(
    //     format!("create{}", model.name),
    //     format!("createOne{}", model.name.clone()),
    // );

    // field(
    //     field_name,
    //     args,
    //     OutputType::object(self.object_type_builder.map_model_object_type(&model)),
    //     Some(SchemaQueryBuilder::ModelQueryBuilder(ModelQueryBuilder::new(
    //         Arc::clone(&model),
    //         QueryTag::CreateOne,
    //         Box::new(|model, parsed_field| {
    //             let mut graph = QueryGraph::new();

    //             write::create_record(&mut graph, model, parsed_field)?;
    //             Ok(graph)
    //         }),
    //     ))),
    // )
    todo!()
}

/// Builds a delete mutation field (e.g. deleteUser) for given model.
fn delete_item_field(ctx: &mut BuilderContext, model: ModelRef) -> Option<Field> {
    // self.argument_builder.delete_arguments(Arc::clone(&model)).map(|args| {
    //     let field_name = self.pluralize_internal(
    //         format!("delete{}", model.name),
    //         format!("deleteOne{}", model.name.clone()),
    //     );

    //     field(
    //         field_name,
    //         args,
    //         OutputType::opt(OutputType::object(
    //             self.object_type_builder.map_model_object_type(&model),
    //         )),
    //         Some(SchemaQueryBuilder::ModelQueryBuilder(ModelQueryBuilder::new(
    //             Arc::clone(&model),
    //             QueryTag::DeleteOne,
    //             Box::new(|model, parsed_field| {
    //                 let mut graph = QueryGraph::new();

    //                 write::delete_record(&mut graph, model, parsed_field)?;
    //                 Ok(graph)
    //             }),
    //         ))),
    //     )
    // })

    todo!()
}

/// Builds a delete many mutation field (e.g. deleteManyUsers) for given model.
fn delete_many_field(ctx: &mut BuilderContext, model: ModelRef) -> Field {
    // let arguments = self.argument_builder.delete_many_arguments(Arc::clone(&model));
    // let field_name = self.pluralize_internal(
    //     format!("deleteMany{}", pluralize(model.name.clone())),
    //     format!("deleteMany{}", model.name.clone()),
    // );

    // field(
    //     field_name,
    //     arguments,
    //     OutputType::object(self.object_type_builder.batch_payload_object_type()),
    //     Some(SchemaQueryBuilder::ModelQueryBuilder(ModelQueryBuilder::new(
    //         Arc::clone(&model),
    //         QueryTag::DeleteMany,
    //         Box::new(|model, parsed_field| {
    //             let mut graph = QueryGraph::new();

    //             write::delete_many_records(&mut graph, model, parsed_field)?;
    //             Ok(graph)
    //         }),
    //     ))),
    // )
    todo!()
}

/// Builds an update mutation field (e.g. updateUser) for given model.
fn update_item_field(ctx: &mut BuilderContext, model: ModelRef) -> Option<Field> {
    // self.argument_builder.update_arguments(Arc::clone(&model)).map(|args| {
    //     let field_name = self.pluralize_internal(format!("update{}", model.name), format!("updateOne{}", model.name));

    //     field(
    //         field_name,
    //         args,
    //         OutputType::opt(OutputType::object(
    //             self.object_type_builder.map_model_object_type(&model),
    //         )),
    //         Some(SchemaQueryBuilder::ModelQueryBuilder(ModelQueryBuilder::new(
    //             Arc::clone(&model),
    //             QueryTag::UpdateOne,
    //             Box::new(|model, parsed_field| {
    //                 let mut graph = QueryGraph::new();

    //                 write::update_record(&mut graph, model, parsed_field)?;
    //                 Ok(graph)
    //             }),
    //         ))),
    //     )
    // })
    todo!()
}

/// Builds an update many mutation field (e.g. updateManyUsers) for given model.
fn update_many_field(ctx: &mut BuilderContext, model: ModelRef) -> Field {
    // let arguments = self.argument_builder.update_many_arguments(Arc::clone(&model));
    // let field_name = self.pluralize_internal(
    //     format!("updateMany{}", pluralize(model.name.clone())),
    //     format!("updateMany{}", model.name.clone()),
    // );

    // field(
    //     field_name,
    //     arguments,
    //     OutputType::object(self.object_type_builder.batch_payload_object_type()),
    //     Some(SchemaQueryBuilder::ModelQueryBuilder(ModelQueryBuilder::new(
    //         Arc::clone(&model),
    //         QueryTag::UpdateMany,
    //         Box::new(|model, parsed_field| {
    //             let mut graph = QueryGraph::new();

    //             write::update_many_records(&mut graph, model, parsed_field)?;
    //             Ok(graph)
    //         }),
    //     ))),
    // )
    todo!()
}

/// Builds an upsert mutation field (e.g. upsertUser) for given model.
fn upsert_item_field(ctx: &mut BuilderContext, model: ModelRef) -> Option<Field> {
    // self.argument_builder.upsert_arguments(Arc::clone(&model)).map(|args| {
    //     let field_name = self.pluralize_internal(format!("upsert{}", model.name), format!("upsertOne{}", model.name));

    //     field(
    //         field_name,
    //         args,
    //         OutputType::object(self.object_type_builder.map_model_object_type(&model)),
    //         Some(SchemaQueryBuilder::ModelQueryBuilder(ModelQueryBuilder::new(
    //             Arc::clone(&model),
    //             QueryTag::UpsertOne,
    //             Box::new(|model, parsed_field| {
    //                 let mut graph = QueryGraph::new();

    //                 write::upsert_record(&mut graph, model, parsed_field)?;
    //                 Ok(graph)
    //             }),
    //         ))),
    //     )
    // })
    todo!()
}
