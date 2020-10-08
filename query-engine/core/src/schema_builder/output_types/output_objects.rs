use super::*;
use prisma_models::ScalarFieldRef;

/// Initializes model output object type cache on the context.
/// This is a critical first step to ensure that all model output object types are present
/// and that subsequent schema computation has a base to rely on.
/// Called only once at the very beginning of schema building.
pub(crate) fn initialize_model_object_type_cache(ctx: &mut BuilderContext) {
    // Compute initial cache. No fields are computed because we first
    // need all models to be present, then we can compute fields in a second pass.
    ctx.internal_data_model
        .models()
        .to_owned()
        .into_iter()
        .for_each(|model| {
            ctx.cache_output_type(
                model.name.clone(),
                Arc::new(ObjectType::new(model.name.clone(), Some(model))),
            )
        });

    // Compute fields on all cached object types.
    ctx.internal_data_model
        .models()
        .to_owned()
        .into_iter()
        .for_each(|model| {
            let obj: ObjectTypeWeakRef = output_objects::map_model_object_type(ctx, &model);
            let fields = compute_model_object_type_fields(ctx, &model);

            obj.into_arc().set_fields(fields);
        });
}

/// Computes model output type fields.
/// Important: This requires that the cache has already been initialized.
fn compute_model_object_type_fields(ctx: &mut BuilderContext, model: &ModelRef) -> Vec<OutputField> {
    model
        .fields()
        .all
        .iter()
        .map(|f| output_objects::map_field(ctx, f))
        .collect()
}

/// Returns an output object type for the given model.
/// Relies on the output type cache being initalized.
pub(crate) fn map_model_object_type(ctx: &mut BuilderContext, model: &ModelRef) -> ObjectTypeWeakRef {
    ctx.get_output_type(&model.name)
        .expect("Invariant violation: Initialized output object type for each model.")
}

pub(crate) fn map_field(ctx: &mut BuilderContext, model_field: &ModelField) -> OutputField {
    field(
        model_field.name(),
        arguments::many_records_field_arguments(ctx, &model_field),
        map_output_type(ctx, &model_field),
        None,
    )
    .optional_if(!model_field.is_required())
}

pub(crate) fn map_output_type(ctx: &mut BuilderContext, model_field: &ModelField) -> OutputType {
    let output_type = match model_field {
        ModelField::Relation(rf) => {
            let related_model_obj = OutputType::object(map_model_object_type(ctx, &rf.related_model()));

            if rf.is_list {
                OutputType::list(related_model_obj)
            } else {
                related_model_obj
            }
        }

        ModelField::Scalar(sf) => match sf.type_identifier {
            TypeIdentifier::String => OutputType::string(),
            TypeIdentifier::Float => OutputType::float(),
            TypeIdentifier::Boolean => OutputType::boolean(),
            TypeIdentifier::Enum(_) => map_enum_field(sf).into(),
            TypeIdentifier::Json => OutputType::json(),
            TypeIdentifier::DateTime => OutputType::date_time(),
            TypeIdentifier::UUID => OutputType::uuid(),
            TypeIdentifier::Int => OutputType::int(),
        },
    };

    if model_field.is_scalar() && model_field.is_list() {
        OutputType::list(output_type)
    } else {
        output_type
    }
}

pub(crate) fn map_enum_field(scalar_field: &ScalarFieldRef) -> EnumType {
    match scalar_field.type_identifier {
        TypeIdentifier::Enum(_) => {
            let internal_enum = scalar_field
                .internal_enum
                .as_ref()
                .expect("Invariant violation: Enum fields are expected to have an internal_enum associated with them.");

            internal_enum.clone().into()
        }
        _ => panic!("Invariant violation: map_enum_field can only be called on scalar enum fields."),
    }
}

pub(crate) fn batch_payload_object_type(ctx: &mut BuilderContext) -> ObjectTypeWeakRef {
    return_cached_output!(ctx, "BatchPayload");

    let object_type = Arc::new(object_type(
        "BatchPayload",
        vec![field("count", vec![], OutputType::int(), None)],
        None,
    ));

    ctx.cache_output_type("BatchPayload".into(), object_type.clone());
    Arc::downgrade(&object_type)
}

/// Builds aggregation object type for given model (e.g. AggregateUser).
pub(crate) fn aggregation_object_type(ctx: &mut BuilderContext, model: &ModelRef) -> ObjectTypeWeakRef {
    let name = format!("Aggregate{}", capitalize(&model.name));
    return_cached_output!(ctx, &name);

    let object = ObjectTypeStrongRef::new(ObjectType::new(&name, Some(ModelRef::clone(model))));
    let mut fields = vec![count_field()];

    append_opt(
        &mut fields,
        numeric_aggregation_field(ctx, "avg", &model, Some(OutputType::float())),
    );

    append_opt(&mut fields, numeric_aggregation_field(ctx, "sum", &model, None));
    append_opt(&mut fields, numeric_aggregation_field(ctx, "min", &model, None));
    append_opt(&mut fields, numeric_aggregation_field(ctx, "max", &model, None));

    object.set_fields(fields);
    ctx.cache_output_type(name, ObjectTypeStrongRef::clone(&object));

    ObjectTypeStrongRef::downgrade(&object)
}

pub(crate) fn count_field() -> OutputField {
    field("count", vec![], OutputType::int(), None)
}

/// Returns an aggregation field with given name if the model contains any numeric fields.
/// Fields inside the object type of the field may have a fixed output type.
pub(crate) fn numeric_aggregation_field(
    ctx: &mut BuilderContext,
    name: &str,
    model: &ModelRef,
    fixed_field_type: Option<OutputType>,
) -> Option<OutputField> {
    let numeric_fields = collect_numeric_fields(model);

    if numeric_fields.is_empty() {
        None
    } else {
        let object_type = OutputType::object(map_numeric_field_aggregation_object(
            ctx,
            model,
            name,
            &numeric_fields,
            fixed_field_type,
        ));

        Some(field(name, vec![], object_type, None).optional())
    }
}

/// Maps the object type for aggregations that operate on a (numeric) field level, rather than the entire model.
/// Fields inside the object may have a fixed output type.
pub(crate) fn map_numeric_field_aggregation_object(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    suffix: &str,
    fields: &[ScalarFieldRef],
    fixed_field_type: Option<OutputType>,
) -> ObjectTypeWeakRef {
    let name = format!("{}{}AggregateOutputType", capitalize(&model.name), capitalize(suffix));
    return_cached_output!(ctx, &name);

    let fields: Vec<OutputField> = fields
        .iter()
        .map(|sf| {
            field(
                sf.name.clone(),
                vec![],
                fixed_field_type
                    .clone()
                    .unwrap_or(map_output_type(ctx, &ModelField::Scalar(sf.clone()))),
                None,
            )
            .optional_if(!sf.is_required)
        })
        .collect();

    let object = Arc::new(object_type(name.clone(), fields, None));
    ctx.cache_output_type(name, object.clone());

    Arc::downgrade(&object)
}

fn collect_numeric_fields(model: &ModelRef) -> Vec<ScalarFieldRef> {
    model
        .fields()
        .scalar()
        .into_iter()
        .filter(|f| match f.type_identifier {
            TypeIdentifier::Int => true,
            TypeIdentifier::Float => true,
            _ => false,
        })
        .collect()
}
