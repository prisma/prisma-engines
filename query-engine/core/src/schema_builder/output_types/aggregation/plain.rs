use prisma_models::ScalarFieldRef;

use super::output_objects::map_scalar_output_type;
use super::*;

/// Builds plain aggregation object type for given model (e.g. AggregateUser).
pub(crate) fn aggregation_object_type(ctx: &mut BuilderContext, model: &ModelRef) -> ObjectTypeWeakRef {
    let ident = Identifier::new(format!("Aggregate{}", capitalize(&model.name)), PRISMA_NAMESPACE);
    return_cached_output!(ctx, &ident);

    let object = ObjectTypeStrongRef::new(ObjectType::new(ident.clone(), Some(ModelRef::clone(model))));
    let mut fields = vec![count_field()];

    append_opt(
        &mut fields,
        numeric_aggregation_field(ctx, "avg", &model, field_avg_output_type),
    );

    append_opt(
        &mut fields,
        numeric_aggregation_field(ctx, "sum", &model, map_scalar_output_type),
    );

    append_opt(
        &mut fields,
        numeric_aggregation_field(ctx, "min", &model, map_scalar_output_type),
    );

    append_opt(
        &mut fields,
        numeric_aggregation_field(ctx, "max", &model, map_scalar_output_type),
    );

    object.set_fields(fields);
    ctx.cache_output_type(ident, ObjectTypeStrongRef::clone(&object));

    ObjectTypeStrongRef::downgrade(&object)
}

pub(crate) fn count_field() -> OutputField {
    field("count", vec![], OutputType::int(), None)
}

/// Returns an aggregation field with given name if the model contains any numeric fields.
/// Fields inside the object type of the field may have a fixed output type.
pub(crate) fn numeric_aggregation_field<F>(
    ctx: &mut BuilderContext,
    name: &str,
    model: &ModelRef,
    type_mapper: F,
) -> Option<OutputField>
where
    F: Fn(&ScalarFieldRef) -> OutputType,
{
    let numeric_fields = collect_numeric_fields(model);

    if numeric_fields.is_empty() {
        None
    } else {
        let object_type = OutputType::object(map_numeric_field_aggregation_object(
            ctx,
            model,
            name,
            &numeric_fields,
            type_mapper,
        ));

        Some(field(name, vec![], object_type, None).optional())
    }
}

/// Maps the object type for aggregations that operate on a (numeric) field level, rather than the entire model.
/// Fields inside the object may have a fixed output type.
pub(crate) fn map_numeric_field_aggregation_object<F>(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    suffix: &str,
    fields: &[ScalarFieldRef],
    type_mapper: F,
) -> ObjectTypeWeakRef
where
    F: Fn(&ScalarFieldRef) -> OutputType,
{
    let ident = Identifier::new(
        format!("{}{}AggregateOutputType", capitalize(&model.name), capitalize(suffix)),
        PRISMA_NAMESPACE,
    );
    return_cached_output!(ctx, &ident);

    let fields: Vec<OutputField> = fields
        .iter()
        .map(|sf| field(sf.name.clone(), vec![], type_mapper(sf), None).optional_if(!sf.is_required))
        .collect();

    let object = Arc::new(object_type(ident.clone(), fields, None));
    ctx.cache_output_type(ident, object.clone());

    Arc::downgrade(&object)
}

fn field_avg_output_type(field: &ScalarFieldRef) -> OutputType {
    match field.type_identifier {
        TypeIdentifier::Int | TypeIdentifier::BigInt | TypeIdentifier::Float => OutputType::float(),
        TypeIdentifier::Decimal => OutputType::decimal(),
        _ => map_scalar_output_type(field),
    }
}

fn collect_numeric_fields(model: &ModelRef) -> Vec<ScalarFieldRef> {
    model
        .fields()
        .scalar()
        .into_iter()
        .filter(|f| {
            matches!(
                f.type_identifier,
                TypeIdentifier::Int | TypeIdentifier::BigInt | TypeIdentifier::Float | TypeIdentifier::Decimal
            )
        })
        .collect()
}
