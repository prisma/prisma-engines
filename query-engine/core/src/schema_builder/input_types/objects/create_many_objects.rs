use super::*;
use datamodel_connector::ConnectorCapability;
use prisma_models::dml::DefaultValue;

/// Create many data input type.
/// Input type allows to write all scalar fields except if in a nested case,
/// where we don't allow the parent scalar to be written (ie. when the relation
/// is inlined on the child).
#[tracing::instrument(skip(ctx, model, parent_field))]
pub(crate) fn create_many_object_type(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> InputObjectTypeWeakRef {
    let name = match parent_field.map(|pf| pf.related_field()) {
        Some(ref f) => format!("{}CreateMany{}Input", model.name, capitalize(f.name.as_str())),
        _ => format!("{}CreateManyInput", model.name),
    };

    let ident = Identifier::new(name, PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    let linking_fields = if let Some(parent_field) = parent_field {
        let child_field = parent_field.related_field();
        if child_field.is_inlined_on_enclosing_model() {
            child_field.linking_fields().scalar_fields().collect()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    // 1) Filter out parent links.
    // 2) Only allow writing autoincrement fields if the connector supports it.
    let scalar_fields: Vec<ScalarFieldRef> = model
        .fields()
        .scalar()
        .into_iter()
        .filter(|sf| {
            if linking_fields.contains(sf) {
                false
            } else if sf.is_autoincrement {
                ctx.capabilities.contains(ConnectorCapability::WritableAutoincField)
            } else {
                true
            }
        })
        .collect();

    let fields = input_fields::scalar_input_fields(
        ctx,
        model.name.clone(),
        "CreateMany",
        scalar_fields,
        |ctx, f: ScalarFieldRef, default: Option<DefaultValue>| {
            let typ = map_scalar_input_type_for_field(ctx, &f);

            input_field(f.name.clone(), typ, default)
                .optional_if(!f.is_required || f.default_value.is_some() || f.is_created_at() || f.is_updated_at())
                .nullable_if(!f.is_required)
        },
        true,
    );

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}
