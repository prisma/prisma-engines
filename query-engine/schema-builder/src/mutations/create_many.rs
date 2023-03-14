use std::sync::Arc;

use crate::{
    capitalize,
    constants::args,
    field, init_input_object_type, input_field,
    input_types::{
        fields::data_input_mapper::{CreateDataInputFieldMapper, DataInputFieldMapper},
        list_union_type,
    },
    output_types::objects,
    BuilderContext, ModelField,
};
use prisma_models::{ModelRef, RelationFieldRef};
use psl::datamodel_connector::ConnectorCapability;
use schema::{Identifier, InputField, InputObjectTypeWeakRef, InputType, OutputField, OutputType, QueryInfo, QueryTag};

/// Builds a create many mutation field (e.g. createManyUsers) for given model.
pub(crate) fn create_many(ctx: &mut BuilderContext, model: &ModelRef) -> Option<OutputField> {
    let arguments = create_many_arguments(ctx, model);
    let field_name = format!("createMany{}", model.name());

    if ctx.has_capability(ConnectorCapability::CreateMany) {
        Some(field(
            field_name,
            arguments,
            OutputType::object(objects::affected_records_object_type(ctx)),
            Some(QueryInfo {
                model: Some(model.clone()),
                tag: QueryTag::CreateMany,
            }),
        ))
    } else {
        None
    }
}

/// Builds "skip_duplicates" and "data" arguments intended for the create many field.
pub(crate) fn create_many_arguments(ctx: &mut BuilderContext, model: &ModelRef) -> Vec<InputField> {
    let create_many_type = InputType::object(create_many_object_type(ctx, model, None));
    let data_arg = input_field(args::DATA, list_union_type(create_many_type, true), None);

    if ctx.has_capability(ConnectorCapability::CreateSkipDuplicates) {
        let skip_arg = input_field(args::SKIP_DUPLICATES, InputType::boolean(), None).optional();

        vec![data_arg, skip_arg]
    } else {
        vec![data_arg]
    }
}

// Create many data input type.
/// Input type allows to write all scalar fields except if in a nested case,
/// where we don't allow the parent scalar to be written (ie. when the relation
/// is inlined on the child).
pub(crate) fn create_many_object_type(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> InputObjectTypeWeakRef {
    let name = match parent_field.map(|pf| pf.related_field()) {
        Some(ref f) => format!("{}CreateMany{}Input", model.name(), capitalize(f.name())),
        _ => format!("{}CreateManyInput", model.name()),
    };

    let ident = Identifier::new_prisma(name);
    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    let filtered_fields = filter_create_many_fields(ctx, model, parent_field);
    let field_mapper = CreateDataInputFieldMapper::new_checked();
    let input_fields = field_mapper.map_all(ctx, filtered_fields);

    input_object.set_fields(input_fields);
    Arc::downgrade(&input_object)
}

/// Filters the given model's fields down to the allowed ones for checked create.
fn filter_create_many_fields<'a>(
    ctx: &BuilderContext,
    model: &'a ModelRef,
    parent_field: Option<&'a RelationFieldRef>,
) -> impl Iterator<Item = ModelField> + 'a {
    let linking_fields = if let Some(parent_field) = parent_field {
        let child_field = parent_field.related_field();
        if child_field.is_inlined_on_enclosing_model() {
            child_field
                .linking_fields()
                .as_scalar_fields()
                .expect("Expected linking fields to be scalar.")
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    let has_create_many = ctx.has_capability(ConnectorCapability::CreateManyWriteableAutoIncId);

    // 1) Filter out parent links.
    // 2) Only allow writing autoincrement fields if the connector supports it.
    model.fields().filter(move |field| match field {
        ModelField::Scalar(sf) => {
            if linking_fields.contains(sf) {
                false
            } else if sf.is_autoincrement() {
                has_create_many
            } else {
                true
            }
        }
        ModelField::Composite(_) => true,
        _ => false,
    })
}
