use super::*;
use crate::enum_types::*;
use input_types::fields::arguments;
use prisma_models::{CompositeFieldRef, ScalarFieldRef};

pub(crate) fn map_output_field(ctx: &mut BuilderContext<'_>, model_field: &ModelField) -> OutputField {
    field(
        model_field.name(),
        arguments::many_records_output_field_arguments(ctx, model_field),
        map_field_output_type(ctx, model_field),
        None,
    )
    .nullable_if(!model_field.is_required())
}

pub(crate) fn map_field_output_type(ctx: &mut BuilderContext<'_>, model_field: &ModelField) -> OutputType {
    match model_field {
        ModelField::Scalar(sf) => map_scalar_output_type_for_field(ctx, sf),
        ModelField::Relation(rf) => map_relation_output_type(ctx, rf),
        ModelField::Composite(cf) => map_composite_field_output_type(ctx, cf),
    }
}

pub(crate) fn map_scalar_output_type_for_field(ctx: &mut BuilderContext<'_>, field: &ScalarFieldRef) -> OutputType {
    map_scalar_output_type(ctx, &field.type_identifier(), field.is_list())
}

pub(crate) fn map_scalar_output_type(ctx: &mut BuilderContext<'_>, typ: &TypeIdentifier, list: bool) -> OutputType {
    let output_type = match typ {
        TypeIdentifier::String => OutputType::string(),
        TypeIdentifier::Float => OutputType::float(),
        TypeIdentifier::Decimal => OutputType::decimal(),
        TypeIdentifier::Boolean => OutputType::boolean(),
        TypeIdentifier::Enum(e) => OutputType::enum_type(map_schema_enum_type(ctx, *e)),
        TypeIdentifier::Json => OutputType::json(),
        TypeIdentifier::DateTime => OutputType::date_time(),
        TypeIdentifier::UUID => OutputType::uuid(),
        TypeIdentifier::Int => OutputType::int(),
        TypeIdentifier::Xml => OutputType::xml(),
        TypeIdentifier::Bytes => OutputType::bytes(),
        TypeIdentifier::BigInt => OutputType::bigint(),
        TypeIdentifier::Unsupported => unreachable!("No unsupported field should reach that path"),
    };

    if list {
        OutputType::list(output_type)
    } else {
        output_type
    }
}

pub(crate) fn map_relation_output_type(ctx: &mut BuilderContext<'_>, rf: &RelationFieldRef) -> OutputType {
    let related_model_obj = OutputType::object(objects::model::map_type(ctx, &rf.related_model()));

    if rf.is_list() {
        OutputType::list(related_model_obj)
    } else {
        related_model_obj
    }
}

fn map_composite_field_output_type(ctx: &mut BuilderContext<'_>, cf: &CompositeFieldRef) -> OutputType {
    let obj = objects::composite::map_type(ctx, &cf.typ());
    let typ = OutputType::Object(obj);

    if cf.is_list() {
        OutputType::list(typ)
    } else {
        typ
    }
}

/// Returns an aggregation field with given name if the passed fields contains any fields.
/// Field types inside the object type of the field are determined by the passed mapper fn.
pub(crate) fn aggregation_relation_field<F, G>(
    ctx: &mut BuilderContext<'_>,
    name: &str,
    model: &ModelRef,
    fields: Vec<RelationFieldRef>,
    type_mapper: F,
    object_mapper: G,
) -> Option<OutputField>
where
    F: Fn(&mut BuilderContext<'_>, &RelationFieldRef) -> OutputType,
    G: Fn(ObjectType) -> ObjectType,
{
    if fields.is_empty() {
        None
    } else {
        let object_type = OutputType::object(map_field_aggration_relation(
            ctx,
            model,
            &fields,
            type_mapper,
            object_mapper,
        ));

        Some(field(name, vec![], object_type, None))
    }
}

/// Maps the object type for aggregations that operate on a field level.
fn map_field_aggration_relation<F, G>(
    ctx: &mut BuilderContext<'_>,
    model: &ModelRef,
    fields: &[RelationFieldRef],
    type_mapper: F,
    object_mapper: G,
) -> ObjectTypeWeakRef
where
    F: Fn(&mut BuilderContext<'_>, &RelationFieldRef) -> OutputType,
    G: Fn(ObjectType) -> ObjectType,
{
    let ident = Identifier::new_prisma(format!("{}CountOutputType", capitalize(model.name())));
    return_cached_output!(ctx, &ident);

    let fields: Vec<OutputField> = fields
        .iter()
        .map(|rf| {
            let mut args = vec![];

            if ctx.has_feature(PreviewFeature::FilteredRelationCount) {
                args.push(arguments::where_argument(ctx, &rf.related_model()))
            }

            field(rf.name(), args, type_mapper(ctx, rf), None)
        })
        .collect();

    let object = object_mapper(object_type(ident.clone(), fields, None));
    let object = Arc::new(object);

    ctx.cache_output_type(ident, object.clone());

    Arc::downgrade(&object)
}
