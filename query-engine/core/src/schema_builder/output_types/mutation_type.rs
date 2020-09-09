use super::*;
use crate::{write, QueryGraph};
use input_types::input_fields;
use prisma_models::{dml, PrismaValue};

/// Builds the root `Mutation` type.
pub(crate) fn build(ctx: &mut BuilderContext) -> (OutputType, ObjectTypeStrongRef) {
    let non_embedded_models = ctx.internal_data_model.non_embedded_models();
    let mut fields: Vec<OutputField> = non_embedded_models
        .into_iter()
        .map(|model| {
            let mut vec = vec![create_item_field(ctx, &model)];

            append_opt(&mut vec, delete_item_field(ctx, &model));
            append_opt(&mut vec, update_item_field(ctx, &model));
            append_opt(&mut vec, upsert_item_field(ctx, &model));

            vec.push(update_many_field(ctx, &model));
            vec.push(delete_many_field(ctx, &model));

            vec
        })
        .flatten()
        .collect();

    create_nested_inputs(ctx);

    if ctx.enable_raw_queries {
        fields.push(create_execute_raw_field());
        fields.push(create_query_raw_field());
    }

    let strong_ref = Arc::new(object_type("Mutation", fields, None));

    (OutputType::Object(Arc::downgrade(&strong_ref)), strong_ref)
}

// implementation note: these need to be in the same function, because these vecs interact: the create inputs will enqueue update inputs, and vice versa.
fn create_nested_inputs(ctx: &mut BuilderContext) {
    let mut nested_create_inputs_queue = std::mem::replace(&mut ctx.nested_create_inputs_queue, Vec::new());
    let mut nested_update_inputs_queue = std::mem::replace(&mut ctx.nested_update_inputs_queue, Vec::new());

    while !(nested_create_inputs_queue.is_empty() && nested_update_inputs_queue.is_empty()) {
        // Create inputs.
        for (input_object, rf) in nested_create_inputs_queue.drain(..) {
            let mut fields = vec![input_fields::nested_create_input_field(ctx, &rf)];
            let nested_connect = input_fields::nested_connect_input_field(ctx, &rf);
            append_opt(&mut fields, nested_connect);

            if feature_flags::get().connectOrCreate {
                let nested_connect_or_create = input_fields::nested_connect_or_create_field(ctx, &rf);
                append_opt(&mut fields, nested_connect_or_create);
            }

            input_object.set_fields(fields);
        }

        // Update inputs.
        for (input_object, rf) in nested_update_inputs_queue.drain(..) {
            let mut fields = vec![input_fields::nested_create_input_field(ctx, &rf)];

            append_opt(&mut fields, input_fields::nested_connect_input_field(ctx, &rf));
            append_opt(&mut fields, input_fields::nested_set_input_field(ctx, &rf));
            append_opt(&mut fields, input_fields::nested_disconnect_input_field(ctx, &rf));
            append_opt(&mut fields, input_fields::nested_delete_input_field(ctx, &rf));
            fields.push(input_fields::nested_update_input_field(ctx, &rf));
            append_opt(&mut fields, input_fields::nested_update_many_field(ctx, &rf));
            append_opt(&mut fields, input_fields::nested_delete_many_field(ctx, &rf));
            append_opt(&mut fields, input_fields::nested_upsert_field(ctx, &rf));

            if feature_flags::get().connectOrCreate {
                append_opt(&mut fields, input_fields::nested_connect_or_create_field(ctx, &rf));
            }

            input_object.set_fields(fields);
        }

        std::mem::swap(&mut nested_create_inputs_queue, &mut ctx.nested_create_inputs_queue);
        std::mem::swap(&mut nested_update_inputs_queue, &mut ctx.nested_update_inputs_queue);
    }
}

fn create_execute_raw_field() -> OutputField {
    field(
        "executeRaw",
        vec![
            input_field("query", InputType::string(), None),
            input_field(
                "parameters",
                InputType::json_list(),
                Some(dml::DefaultValue::Single(PrismaValue::String("[]".into()))),
            )
            .optional(),
        ],
        OutputType::json(),
        None,
    )
}

fn create_query_raw_field() -> OutputField {
    field(
        "queryRaw",
        vec![
            input_field("query", InputType::string(), None),
            input_field(
                "parameters",
                InputType::json_list(),
                Some(dml::DefaultValue::Single(PrismaValue::String("[]".into()))),
            )
            .optional(),
        ],
        OutputType::json(),
        None,
    )
}

/// Builds a create mutation field (e.g. createUser) for given model.
fn create_item_field(ctx: &mut BuilderContext, model: &ModelRef) -> OutputField {
    let args = arguments::create_arguments(ctx, model).unwrap_or_else(|| vec![]);
    let field_name = ctx.pluralize_internal(format!("create{}", model.name), format!("createOne{}", model.name));

    field(
        field_name,
        args,
        OutputType::object(output_objects::map_model_object_type(ctx, &model)),
        Some(SchemaQueryBuilder::ModelQueryBuilder(ModelQueryBuilder::new(
            model.clone(),
            QueryTag::CreateOne,
            Box::new(|model, parsed_field| {
                let mut graph = QueryGraph::new();

                write::create_record(&mut graph, model, parsed_field)?;
                Ok(graph)
            }),
        ))),
    )
}

/// Builds a delete mutation field (e.g. deleteUser) for given model.
fn delete_item_field(ctx: &mut BuilderContext, model: &ModelRef) -> Option<OutputField> {
    arguments::delete_arguments(ctx, model).map(|args| {
        let field_name = ctx.pluralize_internal(format!("delete{}", model.name), format!("deleteOne{}", model.name));

        field(
            field_name,
            args,
            OutputType::object(output_objects::map_model_object_type(ctx, &model)),
            Some(SchemaQueryBuilder::ModelQueryBuilder(ModelQueryBuilder::new(
                model.clone(),
                QueryTag::DeleteOne,
                Box::new(|model, parsed_field| {
                    let mut graph = QueryGraph::new();

                    write::delete_record(&mut graph, model, parsed_field)?;
                    Ok(graph)
                }),
            ))),
        )
        .optional()
    })
}

/// Builds a delete many mutation field (e.g. deleteManyUsers) for given model.
fn delete_many_field(ctx: &mut BuilderContext, model: &ModelRef) -> OutputField {
    let arguments = arguments::delete_many_arguments(ctx, model);
    let field_name = ctx.pluralize_internal(
        format!("deleteMany{}", pluralize(&model.name)),
        format!("deleteMany{}", model.name),
    );

    field(
        field_name,
        arguments,
        OutputType::object(output_objects::batch_payload_object_type(ctx)),
        Some(SchemaQueryBuilder::ModelQueryBuilder(ModelQueryBuilder::new(
            model.clone(),
            QueryTag::DeleteMany,
            Box::new(|model, parsed_field| {
                let mut graph = QueryGraph::new();

                write::delete_many_records(&mut graph, model, parsed_field)?;
                Ok(graph)
            }),
        ))),
    )
}

/// Builds an update mutation field (e.g. updateUser) for given model.
fn update_item_field(ctx: &mut BuilderContext, model: &ModelRef) -> Option<OutputField> {
    arguments::update_arguments(ctx, model).map(|args| {
        let field_name = ctx.pluralize_internal(format!("update{}", model.name), format!("updateOne{}", model.name));

        field(
            field_name,
            args,
            OutputType::object(output_objects::map_model_object_type(ctx, &model)),
            Some(SchemaQueryBuilder::ModelQueryBuilder(ModelQueryBuilder::new(
                model.clone(),
                QueryTag::UpdateOne,
                Box::new(|model, parsed_field| {
                    let mut graph = QueryGraph::new();

                    write::update_record(&mut graph, model, parsed_field)?;
                    Ok(graph)
                }),
            ))),
        )
        .optional()
    })
}

/// Builds an update many mutation field (e.g. updateManyUsers) for given model.
fn update_many_field(ctx: &mut BuilderContext, model: &ModelRef) -> OutputField {
    let arguments = arguments::update_many_arguments(ctx, model);
    let field_name = ctx.pluralize_internal(
        format!("updateMany{}", pluralize(model.name.as_str())),
        format!("updateMany{}", model.name),
    );

    field(
        field_name,
        arguments,
        OutputType::object(output_objects::batch_payload_object_type(ctx)),
        Some(SchemaQueryBuilder::ModelQueryBuilder(ModelQueryBuilder::new(
            model.clone(),
            QueryTag::UpdateMany,
            Box::new(|model, parsed_field| {
                let mut graph = QueryGraph::new();

                write::update_many_records(&mut graph, model, parsed_field)?;
                Ok(graph)
            }),
        ))),
    )
}

/// Builds an upsert mutation field (e.g. upsertUser) for given model.
fn upsert_item_field(ctx: &mut BuilderContext, model: &ModelRef) -> Option<OutputField> {
    arguments::upsert_arguments(ctx, model).map(|args| {
        let field_name = ctx.pluralize_internal(format!("upsert{}", model.name), format!("upsertOne{}", model.name));

        field(
            field_name,
            args,
            OutputType::object(output_objects::map_model_object_type(ctx, &model)),
            Some(SchemaQueryBuilder::ModelQueryBuilder(ModelQueryBuilder::new(
                model.clone(),
                QueryTag::UpsertOne,
                Box::new(|model, parsed_field| {
                    let mut graph = QueryGraph::new();

                    write::upsert_record(&mut graph, model, parsed_field)?;
                    Ok(graph)
                }),
            ))),
        )
    })
}
