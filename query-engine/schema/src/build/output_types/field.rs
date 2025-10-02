use super::*;
use input_types::fields::arguments;
use query_structure::{CompositeFieldRef, ScalarFieldRef};

pub(crate) fn map_output_field(ctx: &'_ QuerySchema, model_field: ModelField) -> OutputField<'_> {
    let cloned_model_field = model_field.clone();
    let model_field_is_required = model_field.is_required();
    field(
        cloned_model_field.borrowed_name(&ctx.internal_data_model.schema),
        move || arguments::many_records_output_field_arguments(ctx, model_field),
        map_field_output_type(ctx, cloned_model_field),
        None,
    )
    .nullable_if(!model_field_is_required)
}

pub(crate) fn map_field_output_type(ctx: &'_ QuerySchema, model_field: ModelField) -> OutputType<'_> {
    match model_field {
        ModelField::Scalar(sf) => map_scalar_output_type_for_field(ctx, sf),
        ModelField::Relation(rf) => map_relation_output_type(ctx, rf),
        ModelField::Composite(cf) => map_composite_field_output_type(ctx, cf),
    }
}

pub(crate) fn map_scalar_output_type_for_field(ctx: &'_ QuerySchema, field: ScalarFieldRef) -> OutputType<'_> {
    map_scalar_output_type(ctx, &field.type_identifier(), field.is_list())
}

pub(crate) fn map_scalar_output_type<'a>(ctx: &'a QuerySchema, typ: &TypeIdentifier, list: bool) -> OutputType<'a> {
    let output_type = match typ {
        TypeIdentifier::String => OutputType::string(),
        TypeIdentifier::Float => OutputType::float(),
        TypeIdentifier::Decimal => OutputType::decimal(),
        TypeIdentifier::Boolean => OutputType::boolean(),
        TypeIdentifier::Enum(e) => OutputType::enum_type(map_schema_enum_type(ctx, *e)),
        TypeIdentifier::Extension(_) => unreachable!("No extension field should reach this path"),
        TypeIdentifier::Json => OutputType::json(),
        TypeIdentifier::DateTime => OutputType::date_time(),
        TypeIdentifier::UUID => OutputType::uuid(),
        TypeIdentifier::Int => OutputType::int(),
        TypeIdentifier::Bytes => OutputType::bytes(),
        TypeIdentifier::BigInt => OutputType::bigint(),
        TypeIdentifier::Unsupported => unreachable!("No unsupported field should reach this path"),
    };

    if list {
        OutputType::list(output_type)
    } else {
        OutputType::non_list(output_type)
    }
}

pub(crate) fn map_relation_output_type(ctx: &'_ QuerySchema, rf: RelationFieldRef) -> OutputType<'_> {
    let related_model_obj = InnerOutputType::Object(objects::model::model_object_type(ctx, rf.related_model()));

    if rf.is_list() {
        OutputType::list(related_model_obj)
    } else {
        OutputType::non_list(related_model_obj)
    }
}

fn map_composite_field_output_type(ctx: &'_ QuerySchema, cf: CompositeFieldRef) -> OutputType<'_> {
    let obj = objects::composite::composite_object_type(ctx, cf.typ());
    let typ = InnerOutputType::Object(obj);

    if cf.is_list() {
        OutputType::list(typ)
    } else {
        OutputType::non_list(typ)
    }
}

/// Returns an aggregation field with given name if the passed fields contains any fields.
/// Field types inside the object type of the field are determined by the passed mapper fn.
pub(crate) fn aggregation_relation_field<'a, F, G>(
    ctx: &'a QuerySchema,
    name: &'a str,
    model: &Model,
    fields: Vec<RelationFieldRef>,
    type_mapper: F,
    object_mapper: G,
) -> Option<OutputField<'a>>
where
    F: Fn(&'a QuerySchema, &RelationFieldRef) -> OutputType<'a> + Send + Sync + 'static,
    G: Fn(ObjectType<'a>) -> ObjectType<'a>,
{
    if fields.is_empty() {
        None
    } else {
        let object_type = OutputType::object(map_field_aggration_relation(
            ctx,
            model,
            fields,
            type_mapper,
            object_mapper,
        ));

        Some(field_no_arguments(name, object_type, None))
    }
}

/// Maps the object type for aggregations that operate on a field level.
fn map_field_aggration_relation<'a, F, G>(
    ctx: &'a QuerySchema,
    model: &Model,
    fields: Vec<RelationFieldRef>,
    type_mapper: F,
    object_mapper: G,
) -> ObjectType<'a>
where
    F: Fn(&'a QuerySchema, &RelationFieldRef) -> OutputType<'a> + Send + Sync + 'static,
    G: Fn(ObjectType<'a>) -> ObjectType<'a>,
{
    let ident = Identifier::new_prisma(format!("{}CountOutputType", capitalize(model.name())));

    object_mapper(ObjectType::new(ident, move || {
        fields
            .clone()
            .into_iter()
            .map(|rf| {
                let cloned_rf = rf.clone();
                field(
                    cloned_rf.borrowed_name(&ctx.internal_data_model.schema),
                    move || {
                        let args = vec![arguments::where_argument(ctx, &rf.related_model())];
                        args
                    },
                    type_mapper(ctx, &cloned_rf),
                    None,
                )
            })
            .collect()
    }))
}
