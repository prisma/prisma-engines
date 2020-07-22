use super::*;

pub(crate) fn build(ctx: &mut BuilderContext) -> (OutputType, ObjectTypeStrongRef) {
    // let non_embedded_models = self.non_embedded_models();
    //     let fields = non_embedded_models
    //         .into_iter()
    //         .map(|m| {
    //             let mut vec = vec![
    //                 self.all_items_field(Arc::clone(&m)),
    //                 self.aggregation_field(Arc::clone(&m)),
    //             ];

    //             append_opt(&mut vec, self.single_item_field(Arc::clone(&m)));
    //             vec
    //         })
    //         .flatten()
    //         .collect();

    //     let strong_ref = Arc::new(object_type("Query", fields, None));

    //     (OutputType::Object(Arc::downgrade(&strong_ref)), strong_ref)
    todo!()
}

/// Builds a "single" query arity item field (e.g. "user", "post" ...) for given model.
fn single_item_field(ctx: &mut BuilderContext, model: ModelRef) -> Option<Field> {
    // self.argument_builder
    //     .where_unique_argument(Arc::clone(&model))
    //     .map(|arg| {
    //         let field_name =
    //             self.pluralize_internal(camel_case(model.name.clone()), format!("findOne{}", model.name.clone()));

    //         field(
    //             field_name,
    //             vec![arg],
    //             OutputType::opt(OutputType::object(
    //                 self.object_type_builder.map_model_object_type(&model),
    //             )),
    //             Some(SchemaQueryBuilder::ModelQueryBuilder(ModelQueryBuilder::new(
    //                 Arc::clone(&model),
    //                 QueryTag::FindOne,
    //                 Box::new(|model, parsed_field| {
    //                     let mut graph = QueryGraph::new();
    //                     let query = ReadOneRecordBuilder::new(parsed_field, model).build()?;

    //                     // Todo: This (and all following query graph validations) should be unified in the query graph builders mod.
    //                     // callers should not have to care about calling validations explicitly.
    //                     graph.create_node(Query::Read(query));
    //                     Ok(graph)
    //                 }),
    //             ))),
    //         )
    //     })
    todo!()
}

/// Builds a "multiple" query arity items field (e.g. "users", "posts", ...) for given model.
fn all_items_field(ctx: &mut BuilderContext, model: ModelRef) -> Field {
    // let args = self.object_type_builder.many_records_arguments(&model);
    // let field_name = self.pluralize_internal(
    //     camel_case(pluralize(model.name.clone())),
    //     format!("findMany{}", model.name.clone()),
    // );

    // field(
    //     field_name,
    //     args,
    //     OutputType::list(OutputType::object(
    //         self.object_type_builder.map_model_object_type(&model),
    //     )),
    //     Some(SchemaQueryBuilder::ModelQueryBuilder(ModelQueryBuilder::new(
    //         Arc::clone(&model),
    //         QueryTag::FindMany,
    //         Box::new(|model, parsed_field| {
    //             let mut graph = QueryGraph::new();
    //             let query = ReadManyRecordsBuilder::new(parsed_field, model).build()?;

    //             graph.create_node(Query::Read(query));
    //             Ok(graph)
    //         }),
    //     ))),
    // )
    todo!()
}

/// Builds an "aggregate" query field (e.g. "aggregateUser") for given model.
fn aggregation_field(ctx: &mut BuilderContext, model: ModelRef) -> Field {
    // let args = self.object_type_builder.many_records_arguments(&model);
    // let field_name = ctx.pluralize_internal(
    //     format!("aggregate{}", model.name.clone()), // Has no legacy counterpart.
    //     format!("aggregate{}", model.name.clone()),
    // );

    // field(
    //     field_name,
    //     args,
    //     OutputType::object(self.object_type_builder.aggregation_object_type(&model)),
    //     Some(SchemaQueryBuilder::ModelQueryBuilder(ModelQueryBuilder::new(
    //         Arc::clone(&model),
    //         QueryTag::Aggregate,
    //         Box::new(|model, parsed_field| {
    //             let mut graph = QueryGraph::new();
    //             let query = AggregateRecordsBuilder::new(parsed_field, model).build()?;

    //             graph.create_node(Query::Read(query));
    //             Ok(graph)
    //         }),
    //     ))),
    // )

    todo!()
}
