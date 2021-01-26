use super::output_objects::map_scalar_output_type_for_field;
use super::*;
use prisma_models::ScalarFieldRef;

pub(crate) mod group_by;
pub(crate) mod plain;

fn field_avg_output_type(ctx: &mut BuilderContext, field: &ScalarFieldRef) -> OutputType {
    match field.type_identifier {
        TypeIdentifier::Int | TypeIdentifier::BigInt | TypeIdentifier::Float => OutputType::float(),
        TypeIdentifier::Decimal => OutputType::decimal(),
        _ => map_scalar_output_type_for_field(ctx, field),
    }
}

fn collect_non_list_nor_json_fields(model: &ModelRef) -> Vec<ScalarFieldRef> {
    model
        .fields()
        .scalar()
        .into_iter()
        .filter(|f| !f.is_list && f.type_identifier != TypeIdentifier::Json)
        .collect()
}

fn collect_numeric_fields(model: &ModelRef) -> Vec<ScalarFieldRef> {
    model
        .fields()
        .scalar()
        .into_iter()
        .filter(|field| field.is_numeric())
        .collect()
}

/// Returns an aggregation field with given name if the passed fields contains any fields.
/// Field types inside the object type of the field are determined by the passed mapper fn.
fn aggregation_field<F, G>(
    ctx: &mut BuilderContext,
    name: &str,
    model: &ModelRef,
    fields: Vec<ScalarFieldRef>,
    type_mapper: F,
    object_mapper: G,
) -> Option<OutputField>
where
    F: Fn(&mut BuilderContext, &ScalarFieldRef) -> OutputType,
    G: Fn(ObjectType) -> ObjectType,
{
    if fields.is_empty() {
        None
    } else {
        let object_type = OutputType::object(map_field_aggregation_object(
            ctx,
            model,
            name,
            &fields,
            type_mapper,
            object_mapper,
        ));

        Some(field(name, vec![], object_type, None).nullable())
    }
}

/// Maps the object type for aggregations that operate on a field level.
fn map_field_aggregation_object<F, G>(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    suffix: &str,
    fields: &[ScalarFieldRef],
    type_mapper: F,
    object_mapper: G,
) -> ObjectTypeWeakRef
where
    F: Fn(&mut BuilderContext, &ScalarFieldRef) -> OutputType,
    G: Fn(ObjectType) -> ObjectType,
{
    let ident = Identifier::new(
        format!("{}{}AggregateOutputType", capitalize(&model.name), capitalize(suffix)),
        PRISMA_NAMESPACE,
    );
    return_cached_output!(ctx, &ident);

    // Non-numerical fields are always set as nullable
    // This is because when there's no data, doing aggregation on them will return NULL
    let fields: Vec<OutputField> = fields
        .iter()
        .map(|sf| {
            field(sf.name.clone(), vec![], type_mapper(ctx, sf), None).nullable_if(!sf.is_required || !sf.is_numeric())
        })
        .collect();

    let object = object_mapper(object_type(ident.clone(), fields, None));
    let object = Arc::new(object);

    ctx.cache_output_type(ident, object.clone());

    Arc::downgrade(&object)
}
