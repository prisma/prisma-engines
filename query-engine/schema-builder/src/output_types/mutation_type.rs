use super::*;
use crate::mutations::{create_many, create_one};
use input_types::fields::{arguments, input_fields};
use prisma_models::{DefaultKind, PrismaValue};
use psl::datamodel_connector::ConnectorCapability;

/// Builds the root `Mutation` type.
pub(crate) fn build(ctx: &mut BuilderContext<'_>) -> OutputObjectTypeId {
    let mut fields = Vec::with_capacity(ctx.internal_data_model.schema.db.models_count());

    for model in ctx.internal_data_model.models() {
        if model.supports_create_operation() {
            fields.push(create_one(ctx, &model));

            append_opt(&mut fields, upsert_item_field(ctx, &model));
            append_opt(&mut fields, create_many(ctx, &model));
        }

        append_opt(&mut fields, delete_item_field(ctx, &model));
        append_opt(&mut fields, update_item_field(ctx, &model));

        fields.push(update_many_field(ctx, &model));
        fields.push(delete_many_field(ctx, &model));
    }

    create_nested_inputs(ctx);

    if ctx.enable_raw_queries && ctx.has_capability(ConnectorCapability::SqlQueryRaw) {
        fields.push(create_execute_raw_field(ctx));
        fields.push(create_query_raw_field(ctx));
    }

    if ctx.enable_raw_queries && ctx.has_capability(ConnectorCapability::MongoDbQueryRaw) {
        fields.push(create_mongodb_run_command_raw(ctx));
    }

    let ident = Identifier::new_prisma("Mutation".to_owned());
    ctx.db.push_output_object_type(object_type(ident, fields, None))
}

// implementation note: these need to be in the same function, because these vecs interact: the create inputs will enqueue update inputs, and vice versa.
fn create_nested_inputs(ctx: &mut BuilderContext<'_>) {
    let mut nested_create_inputs_queue = std::mem::take(&mut ctx.nested_create_inputs_queue);
    let mut nested_update_inputs_queue = std::mem::take(&mut ctx.nested_update_inputs_queue);

    while !(nested_create_inputs_queue.is_empty() && nested_update_inputs_queue.is_empty()) {
        // Create inputs.
        for (input_object, rf) in nested_create_inputs_queue.drain(..) {
            if rf.related_model().supports_create_operation() {
                let nested_create_one_field = input_fields::nested_create_one_input_field(ctx, &rf);
                ctx.db.push_input_field(input_object, nested_create_one_field);

                if let Some(field) = input_fields::nested_connect_or_create_field(ctx, &rf) {
                    ctx.db.push_input_field(input_object, field);
                }

                if let Some(field) = input_fields::nested_create_many_input_field(ctx, &rf) {
                    ctx.db.push_input_field(input_object, field);
                }
            }

            let nested_connect_input_field = input_fields::nested_connect_input_field(ctx, &rf);
            ctx.db.push_input_field(input_object, nested_connect_input_field);
        }

        // Update inputs.
        for (input_object, rf) in nested_update_inputs_queue.drain(..) {
            let mut fields = vec![];

            if rf.related_model().supports_create_operation() {
                fields.push(input_fields::nested_create_one_input_field(ctx, &rf));

                append_opt(&mut fields, input_fields::nested_connect_or_create_field(ctx, &rf));
                append_opt(&mut fields, input_fields::nested_upsert_field(ctx, &rf));
                append_opt(&mut fields, input_fields::nested_create_many_input_field(ctx, &rf));
            }

            append_opt(&mut fields, input_fields::nested_set_input_field(ctx, &rf));
            append_opt(&mut fields, input_fields::nested_disconnect_input_field(ctx, &rf));
            append_opt(&mut fields, input_fields::nested_delete_input_field(ctx, &rf));

            fields.push(input_fields::nested_connect_input_field(ctx, &rf));
            fields.push(input_fields::nested_update_input_field(ctx, &rf));

            append_opt(&mut fields, input_fields::nested_update_many_field(ctx, &rf));
            append_opt(&mut fields, input_fields::nested_delete_many_field(ctx, &rf));

            for field in fields {
                ctx.db.push_input_field(input_object, field);
            }
        }

        std::mem::swap(&mut nested_create_inputs_queue, &mut ctx.nested_create_inputs_queue);
        std::mem::swap(&mut nested_update_inputs_queue, &mut ctx.nested_update_inputs_queue);
    }
}

fn create_execute_raw_field(ctx: &mut BuilderContext<'_>) -> OutputField {
    field(
        "executeRaw",
        vec![
            input_field(ctx, "query", InputType::string(), None),
            input_field(
                ctx,
                "parameters",
                InputType::json_list(),
                Some(DefaultKind::Single(PrismaValue::String("[]".into()))),
            )
            .optional(),
        ],
        OutputType::json(),
        Some(QueryInfo {
            tag: QueryTag::ExecuteRaw,
            model: None,
        }),
    )
}

fn create_query_raw_field(ctx: &mut BuilderContext<'_>) -> OutputField {
    field(
        "queryRaw",
        vec![
            input_field(ctx, "query", InputType::string(), None),
            input_field(
                ctx,
                "parameters",
                InputType::json_list(),
                Some(DefaultKind::Single(PrismaValue::String("[]".into()))),
            )
            .optional(),
        ],
        OutputType::json(),
        Some(QueryInfo {
            tag: QueryTag::QueryRaw,
            model: None,
        }),
    )
}

fn create_mongodb_run_command_raw(ctx: &mut BuilderContext<'_>) -> OutputField {
    field(
        "runCommandRaw",
        vec![input_field(ctx, "command", InputType::json(), None)],
        OutputType::json(),
        Some(QueryInfo {
            tag: QueryTag::RunCommandRaw,
            model: None,
        }),
    )
}

/// Builds a delete mutation field (e.g. deleteUser) for given model.
fn delete_item_field(ctx: &mut BuilderContext<'_>, model: &ModelRef) -> Option<OutputField> {
    arguments::delete_one_arguments(ctx, model).map(|args| {
        let field_name = format!("deleteOne{}", model.name());

        field(
            field_name,
            args,
            OutputType::object(objects::model::map_type(ctx, model)),
            Some(QueryInfo {
                model: Some(model.clone()),
                tag: QueryTag::DeleteOne,
            }),
        )
        .nullable()
    })
}

/// Builds a delete many mutation field (e.g. deleteManyUsers) for given model.
fn delete_many_field(ctx: &mut BuilderContext<'_>, model: &ModelRef) -> OutputField {
    let arguments = arguments::delete_many_arguments(ctx, model);
    let field_name = format!("deleteMany{}", model.name());

    field(
        field_name,
        arguments,
        OutputType::object(objects::affected_records_object_type(ctx)),
        Some(QueryInfo {
            model: Some(model.clone()),
            tag: QueryTag::DeleteMany,
        }),
    )
}

/// Builds an update mutation field (e.g. updateUser) for given model.
fn update_item_field(ctx: &mut BuilderContext<'_>, model: &ModelRef) -> Option<OutputField> {
    arguments::update_one_arguments(ctx, model).map(|args| {
        let field_name = format!("updateOne{}", model.name());

        field(
            field_name,
            args,
            OutputType::object(objects::model::map_type(ctx, model)),
            Some(QueryInfo {
                model: Some(model.clone()),
                tag: QueryTag::UpdateOne,
            }),
        )
        .nullable()
    })
}

/// Builds an update many mutation field (e.g. updateManyUsers) for given model.
fn update_many_field(ctx: &mut BuilderContext<'_>, model: &ModelRef) -> OutputField {
    let arguments = arguments::update_many_arguments(ctx, model);
    let field_name = format!("updateMany{}", model.name());

    field(
        field_name,
        arguments,
        OutputType::object(objects::affected_records_object_type(ctx)),
        Some(QueryInfo {
            model: Some(model.clone()),
            tag: QueryTag::UpdateMany,
        }),
    )
}

/// Builds an upsert mutation field (e.g. upsertUser) for given model.
fn upsert_item_field(ctx: &mut BuilderContext<'_>, model: &ModelRef) -> Option<OutputField> {
    arguments::upsert_arguments(ctx, model).map(|args| {
        let field_name = format!("upsertOne{}", model.name());

        field(
            field_name,
            args,
            OutputType::object(objects::model::map_type(ctx, model)),
            Some(QueryInfo {
                model: Some(model.clone()),
                tag: QueryTag::UpsertOne,
            }),
        )
    })
}
