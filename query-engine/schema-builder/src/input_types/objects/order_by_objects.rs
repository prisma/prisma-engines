use super::*;
use constants::{aggregations, ordering};
use output_types::aggregation;
use prisma_models::prelude::ParentContainer;

#[derive(Debug, Default)]
pub(crate) struct OrderByOptions {
    pub(crate) include_relations: bool,
    pub(crate) include_scalar_aggregations: bool,
    pub(crate) include_full_text_search: bool,
}

impl OrderByOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_aggregates(mut self) -> Self {
        self.include_scalar_aggregations = true;
        self
    }

    pub fn type_suffix(&self) -> &'static str {
        match (
            self.include_relations,
            self.include_scalar_aggregations,
            self.include_full_text_search,
        ) {
            (true, false, false) => "WithRelation",
            (false, true, false) => "WithAggregation",
            (true, false, true) => "WithRelationAndSearchRelevance",
            _ => "",
        }
    }
}

/// Builds "<Container>OrderBy<Suffixes>Input" object types.
pub(crate) fn order_by_object_type(
    ctx: &mut BuilderContext,
    container: &ParentContainer,
    options: &OrderByOptions,
) -> InputObjectTypeWeakRef {
    let ident = Identifier::new_prisma(format!("{}OrderBy{}Input", container.name(), options.type_suffix()));
    return_cached_input!(ctx, &ident);

    let mut input_object = init_input_object_type(ident.clone());
    input_object.require_at_most_one_field();

    let input_object = Arc::new(input_object);
    ctx.cache_input_type(ident, input_object.clone());

    // Basic orderBy fields.
    let mut fields: Vec<_> = container
        .fields()
        .iter()
        .filter_map(|field| match field {
            // We exclude composites if we're in aggregations land (groupBy).
            ModelField::Composite(_) if options.include_scalar_aggregations => None,
            _ => orderby_field_mapper(field, ctx, options),
        })
        .collect();

    if options.include_scalar_aggregations {
        // orderBy Fields for aggregation orderings.
        fields.extend(compute_scalar_aggregation_fields(ctx, container));
    }

    if options.include_full_text_search {
        // orderBy Fields for full text searches.
        append_opt(&mut fields, order_by_field_text_search(ctx, container))
    }

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}

fn compute_scalar_aggregation_fields(ctx: &mut BuilderContext, container: &ParentContainer) -> Vec<InputField> {
    let non_list_nor_json_fields = aggregation::collect_non_list_nor_json_fields(container);
    let numeric_fields = aggregation::collect_numeric_fields(container);
    let scalar_fields = container
        .fields()
        .into_iter()
        .flat_map(ModelField::into_scalar)
        .collect::<Vec<ScalarFieldRef>>();

    let fields = vec![
        order_by_field_aggregate(aggregations::UNDERSCORE_COUNT, "Count", ctx, container, scalar_fields),
        order_by_field_aggregate(
            aggregations::UNDERSCORE_AVG,
            "Avg",
            ctx,
            container,
            numeric_fields.clone(),
        ),
        order_by_field_aggregate(
            aggregations::UNDERSCORE_MAX,
            "Max",
            ctx,
            container,
            non_list_nor_json_fields.clone(),
        ),
        order_by_field_aggregate(
            aggregations::UNDERSCORE_MIN,
            "Min",
            ctx,
            container,
            non_list_nor_json_fields,
        ),
        order_by_field_aggregate(aggregations::UNDERSCORE_SUM, "Sum", ctx, container, numeric_fields),
    ];

    fields.into_iter().flatten().collect()
}

fn orderby_field_mapper(field: &ModelField, ctx: &mut BuilderContext, options: &OrderByOptions) -> Option<InputField> {
    match field {
        // To-many relation field.
        ModelField::Relation(rf) if rf.is_list() && options.include_relations => {
            let related_model = rf.related_model();
            let to_many_aggregate_type = order_by_to_many_aggregate_object_type(ctx, &related_model.into());

            Some(input_field(rf.name(), InputType::object(to_many_aggregate_type), None).optional())
        }

        // To-one relation field.
        ModelField::Relation(rf) if options.include_relations => {
            let related_model = rf.related_model();
            let related_object_type = order_by_object_type(ctx, &related_model.into(), options);

            Some(input_field(rf.name(), InputType::object(related_object_type), None).optional())
        }

        // Scalar field.
        ModelField::Scalar(sf) => {
            let mut types = vec![InputType::Enum(sort_order_enum(ctx))];

            if ctx.has_feature(PreviewFeature::OrderByNulls)
                && ctx.has_capability(ConnectorCapability::OrderByNullsFirstLast)
                && !sf.is_required()
                && !sf.is_list()
            {
                types.push(InputType::object(sort_nulls_object_type(ctx)));
            }

            Some(input_field(sf.name(), types, None).optional())
        }

        // Composite field.
        ModelField::Composite(cf) if cf.is_list() => {
            let to_many_aggregate_type = order_by_to_many_aggregate_object_type(ctx, &(cf.typ()).into());
            Some(input_field(cf.name().to_owned(), InputType::object(to_many_aggregate_type), None).optional())
        }

        ModelField::Composite(cf) => {
            let composite_order_object_type = order_by_object_type(ctx, &(cf.typ()).into(), &OrderByOptions::new());

            Some(input_field(cf.name(), InputType::object(composite_order_object_type), None).optional())
        }

        _ => None,
    }
}

fn sort_nulls_object_type(ctx: &mut BuilderContext) -> InputObjectTypeWeakRef {
    let ident = Identifier::new_prisma("SortOrderInput");
    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    let fields = vec![
        input_field(ordering::SORT, InputType::Enum(sort_order_enum(ctx)), None),
        input_field(ordering::NULLS, InputType::Enum(nulls_order_enum(ctx)), None).optional(),
    ];

    input_object.set_fields(fields);

    Arc::downgrade(&input_object)
}

fn order_by_field_aggregate(
    name: &str,
    suffix: &str,
    ctx: &mut BuilderContext,
    container: &ParentContainer,
    scalar_fields: Vec<ScalarFieldRef>,
) -> Option<InputField> {
    if scalar_fields.is_empty() {
        None
    } else {
        Some(
            input_field(
                name,
                InputType::object(order_by_object_type_aggregate(suffix, ctx, container, scalar_fields)),
                None,
            )
            .optional(),
        )
    }
}

fn order_by_object_type_aggregate(
    suffix: &str,
    ctx: &mut BuilderContext,
    container: &ParentContainer,
    scalar_fields: Vec<ScalarFieldRef>,
) -> InputObjectTypeWeakRef {
    let ident = Identifier::new_prisma(format!("{}{}OrderByAggregateInput", container.name(), suffix));

    return_cached_input!(ctx, &ident);

    let mut input_object = init_input_object_type(ident.clone());
    input_object.require_exactly_one_field();

    let input_object = Arc::new(input_object);
    ctx.cache_input_type(ident, input_object.clone());

    let fields = scalar_fields
        .iter()
        .map(|sf| input_field(sf.name(), InputType::Enum(sort_order_enum(ctx)), None).optional())
        .collect();

    input_object.set_fields(fields);
    Arc::downgrade(&input_object)
}

fn order_by_to_many_aggregate_object_type(
    ctx: &mut BuilderContext,
    container: &ParentContainer,
) -> InputObjectTypeWeakRef {
    let container_type = match container {
        ParentContainer::Model(_) => "Relation",
        ParentContainer::CompositeType(_) => "Composite",
    };

    let ident = Identifier::new_prisma(format!("{}OrderBy{}AggregateInput", container.name(), container_type));
    return_cached_input!(ctx, &ident);

    let mut input_object = init_input_object_type(ident.clone());
    input_object.require_exactly_one_field();

    let input_object = Arc::new(input_object);
    ctx.cache_input_type(ident, input_object.clone());

    let fields = vec![input_field(
        aggregations::UNDERSCORE_COUNT,
        InputType::Enum(sort_order_enum(ctx)),
        None,
    )
    .optional()];

    input_object.set_fields(fields);

    Arc::downgrade(&input_object)
}

fn order_by_field_text_search(ctx: &mut BuilderContext, container: &ParentContainer) -> Option<InputField> {
    let scalar_fields: Vec<_> = container
        .fields()
        .into_iter()
        .filter_map(|field| match field {
            ModelField::Scalar(sf) if sf.type_identifier() == TypeIdentifier::String => Some(sf),
            _ => None,
        })
        .collect();

    if scalar_fields.is_empty() {
        None
    } else {
        Some(
            input_field(
                ordering::UNDERSCORE_RELEVANCE,
                InputType::object(order_by_object_type_text_search(ctx, container, scalar_fields)),
                None,
            )
            .optional(),
        )
    }
}

fn order_by_object_type_text_search(
    ctx: &mut BuilderContext,
    container: &ParentContainer,
    scalar_fields: Vec<ScalarFieldRef>,
) -> InputObjectTypeWeakRef {
    let ident = Identifier::new_prisma(format!("{}OrderByRelevanceInput", container.name()));

    return_cached_input!(ctx, &ident);

    let input_object = Arc::new(init_input_object_type(ident.clone()));
    ctx.cache_input_type(ident, input_object.clone());

    let fields_enum_type = InputType::enum_type(order_by_relevance_enum(
        ctx,
        &container.name(),
        scalar_fields.iter().map(|sf| sf.name().to_owned()).collect(),
    ));

    let fields = vec![
        input_field(
            ordering::FIELDS,
            vec![fields_enum_type.clone(), InputType::list(fields_enum_type)],
            None,
        ),
        input_field(ordering::SORT, InputType::Enum(sort_order_enum(ctx)), None),
        input_field(ordering::SEARCH, InputType::string(), None),
    ];

    input_object.set_fields(fields);

    Arc::downgrade(&input_object)
}
