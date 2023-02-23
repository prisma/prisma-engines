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
    let ident = Identifier::new(
        format!("{}ScalarWhere{}Input", model.name(), aggregate),
        PRISMA_NAMESPACE,
    );
    return_cached_input!(ctx, &ident);

    let mut input_object = init_input_object_type(ident.clone());
    input_object.set_tag(ObjectTag::WhereInputType(ParentContainer::Model(Arc::downgrade(model))));

    let input_object = Arc::new(input_object);
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

    input_fields.extend(model.fields().filter_all(|_| true).into_iter().filter_map(|f| match f {
        ModelField::Scalar(_) => Some(input_fields::filter_input_field(ctx, &f, include_aggregates)),
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

    let mut input_object = init_input_object_type(ident.clone());
    input_object.set_tag(ObjectTag::WhereInputType(container.clone()));

    let input_object = Arc::new(input_object);
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
    let ident = Identifier::new(format!("{}WhereUniqueInput", model.name()), PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    // Split unique & ID fields vs all the other fields
    let (unique_fields, rest_fields): (Vec<_>, Vec<_>) = model
        .fields()
        .filter_all(|_| true)
        .into_iter()
        .partition(|f| f.is_scalar() && f.is_unique());
    // @@unique compound fields.
    let compound_uniques: Vec<_> = model
        .unique_indexes()
        .into_iter()
        .filter(|index| index.fields.len() > 1)
        .map(|index| {
            let typ = compound_field_unique_object_type(ctx, model, index.name.as_ref(), index.fields());
            let name = compound_index_field_name(index);

            (name, typ)
        })
        .collect();
    // @@id compound field (there can be only one per model).
    let compound_id = model.fields().compound_id().map(|pk| {
        let name = compound_id_field_name(pk);
        let typ = compound_field_unique_object_type(ctx, model, pk.alias.as_ref(), pk.fields());

        (name, typ)
    });

    // Concatenated list of uniques/@@unique/@@id fields on which the input type constraints should be applied (that at least one of them is set).
    let constrained_fields: Vec<_> = {
        let unique_names = unique_fields.iter().map(|f| f.name().to_owned());
        let compound_unique_names = compound_uniques.iter().map(|(name, _)| name.to_owned());
        let compound_id_name = compound_id.iter().map(|(name, _)| name.to_owned());

        unique_names
            .chain(compound_unique_names)
            .chain(compound_id_name)
            .collect()
    };

    let mut x = init_input_object_type(ident.clone());

    if ctx.has_feature(PreviewFeature::ExtendedWhereUnique) {
        x.require_at_least_one_field();
        x.apply_constraints_on_fields(constrained_fields);
    } else {
        x.require_exactly_one_field();
    }

    let input_object = Arc::new(x);
    ctx.cache_input_type(ident, input_object.clone());

    let mut fields: Vec<InputField> = unique_fields
        .into_iter()
        .map(|f| {
            let sf = f.as_scalar().unwrap();
            let name = sf.name();
            let typ = map_scalar_input_type_for_field(ctx, sf);

            input_field(name, typ, None).optional()
        })
        .collect();

    // @@unique compound fields.
    let compound_unique_fields: Vec<InputField> = compound_uniques
        .into_iter()
        .map(|(name, typ)| input_field(name, InputType::object(typ), None).optional())
        .collect();

    // @@id compound field (there can be only one per model).
    let compound_id_field = compound_id.map(|(name, typ)| input_field(name, InputType::object(typ), None).optional());

    // Boolean operators AND/OR/NOT, which are _not_ where unique inputs
    let where_input_type = InputType::object(where_object_type(ctx, ParentContainer::Model(Arc::downgrade(model))));
    let boolean_operators = vec![
        input_field(
            filters::AND,
            vec![where_input_type.clone(), InputType::list(where_input_type.clone())],
            None,
        )
        .optional(),
        input_field(filters::OR, vec![InputType::list(where_input_type.clone())], None).optional(),
        input_field(
            filters::NOT,
            vec![where_input_type.clone(), InputType::list(where_input_type)],
            None,
        )
        .optional(),
    ];

    let rest_fields: Vec<_> = rest_fields
        .into_iter()
        .map(|f| input_fields::filter_input_field(ctx, &f, false))
        .collect();

    fields.extend(compound_unique_fields);
    fields.extend(compound_id_field);

    if ctx.has_feature(PreviewFeature::ExtendedWhereUnique) {
        fields.extend(boolean_operators);
        fields.extend(rest_fields);
    }

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
            model.name(),
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
            let name = field.name();
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
    let ident = Identifier::new(format!("{}ObjectEqualityInput", cf.typ().name()), PRISMA_NAMESPACE);
    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    let mut fields = vec![];

    let composite_type = cf.typ();
    let input_fields = composite_type.fields().into_iter().map(|f| match f {
        ModelField::Scalar(sf) => input_field(sf.name(), map_scalar_input_type_for_field(ctx, &sf), None)
            .optional_if(!sf.is_required())
            .nullable_if(!sf.is_required() && !sf.is_list()),

        ModelField::Composite(cf) => {
            let types = if cf.is_list() {
                // The object (aka shorthand) syntax is only supported because the client used to expose all
                // list input types as T | T[]. Consider removing it one day.
                list_union_type(InputType::object(composite_equality_object(ctx, &cf)), true)
            } else {
                vec![InputType::object(composite_equality_object(ctx, &cf))]
            };

            input_field(cf.name(), types, None)
                .optional_if(!cf.is_required())
                .nullable_if(!cf.is_required() && !cf.is_list())
        }

        ModelField::Relation(_) => unimplemented!(),
    });

    fields.extend(input_fields);
    input_object.set_fields(fields);

    Arc::downgrade(&input_object)
}
