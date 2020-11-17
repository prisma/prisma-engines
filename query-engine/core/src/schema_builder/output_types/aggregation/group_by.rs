use prisma_models::ScalarFieldRef;

use super::output_objects::map_scalar_output_type;
use super::*;

/// Builds group by aggregation object type for given model (e.g. GroupByUserOutputType).
pub(crate) fn group_by_output_object_type(ctx: &mut BuilderContext, model: &ModelRef) -> ObjectTypeWeakRef {
    let ident = Identifier::new(
        format!("{}GroupByOutputType", capitalize(&model.name)),
        PRISMA_NAMESPACE,
    );
    return_cached_output!(ctx, &ident);

    let object = ObjectTypeStrongRef::new(ObjectType::new(ident.clone(), Some(ModelRef::clone(model))));

    // Model fields that can be grouped by value.
    let mut object_fields = scalar_fields(model);

    // Fields used in aggregations
    let non_list_fields = collect_non_list_fields(model);
    let numeric_fields = collect_numeric_fields(model);

    // Count is available on all fields.
    append_opt(
        &mut object_fields,
        aggregation_field(ctx, "count", &model, model.fields().scalar(), field_avg_output_type),
    );

    append_opt(
        &mut object_fields,
        aggregation_field(ctx, "avg", &model, numeric_fields.clone(), field_avg_output_type),
    );

    append_opt(
        &mut object_fields,
        aggregation_field(ctx, "sum", &model, numeric_fields.clone(), map_scalar_output_type),
    );

    append_opt(
        &mut object_fields,
        aggregation_field(ctx, "min", &model, non_list_fields.clone(), map_scalar_output_type),
    );

    append_opt(
        &mut object_fields,
        aggregation_field(ctx, "max", &model, non_list_fields, map_scalar_output_type),
    );

    object.set_fields(object_fields);
    ctx.cache_output_type(ident, ObjectTypeStrongRef::clone(&object));

    ObjectTypeStrongRef::downgrade(&object)
}

fn scalar_fields(model: &ModelRef) -> Vec<OutputField> {
    let fields = model.fields().scalar();

    fields
        .into_iter()
        .map(|f| field(f.name.clone(), vec![], map_scalar_output_type(&f), None).optional())
        .collect()
}

/// Returns an aggregation field with given name if the passed fields contains any fields.
/// Field types inside the object type of the field are determined by the passed mapper fn.
pub(crate) fn aggregation_field<F>(
    ctx: &mut BuilderContext,
    name: &str,
    model: &ModelRef,
    fields: Vec<ScalarFieldRef>,
    type_mapper: F,
) -> Option<OutputField>
where
    F: Fn(&ScalarFieldRef) -> OutputType,
{
    if fields.is_empty() {
        None
    } else {
        let object_type = OutputType::object(map_field_aggregation_object(ctx, model, name, &fields, type_mapper));

        Some(field(name, vec![], object_type, None).optional())
    }
}

/// Maps the object type for aggregations that operate on a field level, rather than the entire model.
pub(crate) fn map_field_aggregation_object<F>(
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
