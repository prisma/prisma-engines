use super::fields::data_input_mapper::*;
use super::*;
use datamodel_connector::ConnectorCapability;

/// Create many data input type.
/// Input type allows to write all scalar fields except if in a nested case,
/// where we don't allow the parent scalar to be written (ie. when the relation
/// is inlined on the child).
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
    return_if_cached!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    let filtered_fields = filter_create_many_fields(ctx, model, parent_field);
    let field_mapper = CreateDataInputFieldMapper::new_checked();
    let input_fields = field_mapper.map_all(ctx, &filtered_fields);

    input_object.set_fields(input_fields);
    Arc::downgrade(&input_object)
}

/// Filters the given model's fields down to the allowed ones for checked create.
fn filter_create_many_fields(
    ctx: &BuilderContext,
    model: &ModelRef,
    parent_field: Option<&RelationFieldRef>,
) -> Vec<ModelField> {
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

    // 1) Filter out parent links.
    // 2) Only allow writing autoincrement fields if the connector supports it.
    model
        .fields()
        .all
        .iter()
        .filter(|field| match field {
            ModelField::Scalar(sf) => {
                if linking_fields.contains(sf) {
                    false
                } else if sf.is_autoincrement {
                    ctx.capabilities
                        .contains(ConnectorCapability::CreateManyWriteableAutoIncId)
                } else {
                    true
                }
            }
            ModelField::Composite(_) => true,
            _ => false,
        })
        .map(Clone::clone)
        .collect()
}
