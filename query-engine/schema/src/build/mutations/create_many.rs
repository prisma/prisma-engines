use super::*;
use crate::{Identifier, IdentifierType, InputField, InputType, OutputField, OutputType, QueryInfo, QueryTag};
use constants::*;
use input_types::{fields::data_input_mapper::*, list_union_type};
use output_types::{field, objects};
use psl::datamodel_connector::ConnectorCapability;
use query_structure::{Model, RelationFieldRef};

/// Builds a create many mutation field (e.g. createManyUsers) for given model.
pub(crate) fn create_many(ctx: &'_ QuerySchema, model: Model) -> OutputField<'_> {
    let field_name = format!("createMany{}", model.name());
    let model_id = model.id;

    field(
        field_name,
        move || create_many_arguments(ctx, model),
        OutputType::object(objects::affected_records_object_type()),
        Some(QueryInfo {
            model: Some(model_id),
            tag: QueryTag::CreateMany,
        }),
    )
}

/// Builds a create many mutation field (e.g. createManyUsers) for given model.
pub(crate) fn create_many_and_return(ctx: &'_ QuerySchema, model: Model) -> OutputField<'_> {
    let field_name = format!("createMany{}AndReturn", model.name());
    let model_id = model.id;
    let object_type = create_many_and_return_output_type(ctx, model.clone());

    field(
        field_name,
        move || create_many_arguments(ctx, model),
        OutputType::list(InnerOutputType::Object(object_type)),
        Some(QueryInfo {
            model: Some(model_id),
            tag: QueryTag::CreateManyAndReturn,
        }),
    )
}

pub(crate) fn create_many_and_return_output_type(ctx: &'_ QuerySchema, model: Model) -> ObjectType<'_> {
    let model_id = model.id;
    let mut obj = ObjectType::new(
        Identifier::new_model(IdentifierType::CreateManyAndReturnOutput(model.clone())),
        move || {
            let mut fields: Vec<_> = model
                .fields()
                .scalar()
                .map(|sf| field::map_output_field(ctx, sf.into()))
                .collect();

            // If the relation is inlined in the enclosing model, that means the foreign keys can be set at creation
            // and thus it makes sense to enable querying this relation.
            for rf in model.fields().relation() {
                if rf.is_inlined_on_enclosing_model() {
                    fields.push(field::map_output_field(ctx, rf.into()));
                }
            }

            fields
        },
    );

    obj.model = Some(model_id);
    obj
}

/// Builds "skip_duplicates" and "data" arguments intended for the create many field.
pub(crate) fn create_many_arguments(ctx: &'_ QuerySchema, model: Model) -> Vec<InputField<'_>> {
    let create_many_type = InputType::object(create_many_object_type(ctx, model, None));
    let data_arg = input_field(args::DATA, list_union_type(create_many_type, true), None);

    if ctx.has_capability(ConnectorCapability::CreateSkipDuplicates) {
        let skip_arg = input_field(args::SKIP_DUPLICATES, vec![InputType::boolean()], None).optional();

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
    ctx: &'_ QuerySchema,
    model: Model,
    parent_field: Option<RelationFieldRef>,
) -> InputObjectType<'_> {
    let ident = Identifier::new_prisma(IdentifierType::CreateManyInput(
        model.clone(),
        parent_field.as_ref().map(|pf| pf.related_field()),
    ));

    let mut input_object = init_input_object_type(ident);
    input_object.set_fields(move || {
        let mut filtered_fields = filter_create_many_fields(ctx, &model, parent_field.clone());
        let field_mapper = CreateDataInputFieldMapper::new_checked();
        field_mapper.map_all(ctx, &mut filtered_fields)
    });
    input_object
}

/// Filters the given model's fields down to the allowed ones for checked create.
fn filter_create_many_fields<'a>(
    ctx: &'a QuerySchema,
    model: &'a Model,
    parent_field: Option<RelationFieldRef>,
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
    let fields = model.fields();

    // 1) Filter out parent links.
    // 2) Only allow writing autoincrement fields if the connector supports it.
    fields.filter_all(move |field| match field {
        ModelField::Scalar(sf) => {
            if linking_fields.contains(sf) {
                false
            } else if sf.is_autoincrement() {
                ctx.has_capability(ConnectorCapability::CreateManyWriteableAutoIncId)
            } else {
                true
            }
        }
        ModelField::Composite(_) => true,
        _ => false,
    })
}
