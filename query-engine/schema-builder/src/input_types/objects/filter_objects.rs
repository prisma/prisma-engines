use super::*;
use constants::filters;
use prisma_models::{prelude::ParentContainer, CompositeFieldRef};
use std::sync::Arc;

pub(crate) fn scalar_filter_object_type(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    include_aggregates: bool,
) -> InputObjectTypeWeakRef {
    let aggregate = if include_aggregates { "WithAggregates" } else { "" };
    let ident = Identifier::new(format!("{}ScalarWhere{}Input", model.name, aggregate), PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    let weak_ref = Arc::downgrade(&input_object);
    let object_type = InputType::object(weak_ref.clone());

    let mut input_fields = vec![
        input_field(
            filters::AND,
            vec![object_type.clone(), InputType::list(object_type.clone())],
            None,
        )
        .optional(),
        input_field(filters::OR, vec![InputType::list(object_type.clone())], None).optional(),
        input_field(
            filters::NOT,
            vec![object_type.clone(), InputType::list(object_type)],
            None,
        )
        .optional(),
    ];

    input_fields.extend(model.fields().all.iter().filter_map(|f| match f {
        ModelField::Scalar(_) => Some(input_fields::filter_input_field(ctx, f, include_aggregates)),
        ModelField::Relation(_) => None,
        ModelField::Composite(_) => None, // [Composites] todo
    }));

    input_object.set_fields(input_fields);
    weak_ref
}

pub(crate) fn where_object_type<T>(ctx: &mut BuilderContext, container: T) -> InputObjectTypeWeakRef
where
    T: Into<ParentContainer>,
{
    let container = container.into();
    let ident = Identifier::new(format!("{}WhereInput", container.name()), PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    let weak_ref = Arc::downgrade(&input_object);
    let object_type = InputType::object(weak_ref.clone());

    let mut fields = vec![
        input_field(
            filters::AND,
            vec![object_type.clone(), InputType::list(object_type.clone())],
            None,
        )
        .optional(),
        input_field(filters::OR, vec![InputType::list(object_type.clone())], None).optional(),
        input_field(
            filters::NOT,
            vec![object_type.clone(), InputType::list(object_type)],
            None,
        )
        .optional(),
    ];

    let input_fields = container
        .fields()
        .into_iter()
        .map(|f| input_fields::filter_input_field(ctx, &f, false));

    fields.extend(input_fields);

    input_object.set_fields(fields);
    weak_ref
}

pub(crate) fn where_unique_object_type(ctx: &mut BuilderContext, model: &ModelRef) -> InputObjectTypeWeakRef {
    let ident = Identifier::new(format!("{}WhereUniqueInput", model.name), PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let mut x = init_input_object_type(ident.clone());
    x.require_exactly_one_field();

    let input_object = Arc::new(x);
    ctx.cache_input_type(ident, input_object.clone());

    // TODO (dom): This can probably be collapsed into just uniques and pks
    // Single unique or ID fields.
    let unique_fields = model.fields().scalar().into_iter().filter(|f| f.unique());

    let mut fields: Vec<InputField> = unique_fields
        .map(|sf| {
            let name = sf.name.clone();
            let typ = map_scalar_input_type_for_field(ctx, &sf);

            input_field(name, typ, None).optional()
        })
        .collect();

    // TODO: problem 1, remove the index.fields.len limitation (this conflicts with @@unique([location.address]))
    // @@unique compound fields.
    let compound_unique_fields: Vec<InputField> = model
        .unique_indexes()
        .into_iter()
        .filter(|index| index.fields.len() > 1)
        .map(|index| {
            let typ = compound_field_unique_object_type(ctx, model, index.name.as_ref(), index.fields());
            let name = compound_index_field_name(index);

            input_field(name, InputType::object(typ), None).optional()
        })
        .collect();

    // @@id compound field (there can be only one per model).
    let compound_id_field = model.fields().compound_id().map(|pk| {
        let name = compound_id_field_name(pk);
        let typ = compound_field_unique_object_type(ctx, model, pk.alias.as_ref(), pk.fields());

        input_field(name, InputType::object(typ), None).optional()
    });

    fields.extend(compound_unique_fields);
    fields.extend(compound_id_field);

    input_object.set_fields(fields);

    Arc::downgrade(&input_object)
}

/// Generates and caches an input object type for a compound field.
fn compound_field_unique_object_type(
    ctx: &mut BuilderContext,
    model: &ModelRef,
    alias: Option<&String>,
    from_fields: Vec<ScalarFieldRef>,
) -> InputObjectTypeWeakRef {
    let ident = Identifier::new(
        format!(
            "{}{}CompoundUniqueInput",
            model.name,
            compound_object_name(alias, &from_fields)
        ),
        PRISMA_NAMESPACE,
    );

    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    let object_fields = from_fields
        .into_iter()
        .map(|field| {
            let name = field.name.clone();
            let typ = map_scalar_input_type_for_field(ctx, &field);

            input_field(name, typ, None)
        })
        .collect();

    input_object.set_fields(object_fields);
    Arc::downgrade(&input_object)
}

/// Object used for full composite equality, e.g. `{ field: "value", field2: 123 } == { field: "value" }`.
/// If the composite is a list, only lists are allowed for comparison, no shorthands are used.
pub(crate) fn composite_equality_object(ctx: &mut BuilderContext, cf: &CompositeFieldRef) -> InputObjectTypeWeakRef {
    let ident = Identifier::new(format!("{}ObjectEqualityInput", cf.typ.name), PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    let mut fields = vec![];

    let input_fields = cf.typ.fields().iter().map(|f| match f {
        ModelField::Scalar(sf) => input_field(sf.name.clone(), map_scalar_input_type_for_field(ctx, sf), None)
            .optional_if(!sf.is_required())
            .nullable_if(!sf.is_required() && !sf.is_list()),

        ModelField::Composite(cf) => {
            let mut types = vec![];

            if cf.is_list() {
                types.push(InputType::list(InputType::object(composite_equality_object(ctx, cf))));
            } else {
                types.push(InputType::object(composite_equality_object(ctx, cf)));
            }

            input_field(cf.name.clone(), types, None)
                .optional_if(!cf.is_required())
                .nullable_if(!cf.is_required() && !cf.is_list())
        }

        ModelField::Relation(_) => unimplemented!(),
    });

    fields.extend(input_fields);
    input_object.set_fields(fields);

    Arc::downgrade(&input_object)
}
