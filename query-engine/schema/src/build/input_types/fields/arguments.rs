use super::*;
use constants::args;
use input_types::objects::order_by_objects::OrderByOptions;
use mutations::create_one;
use objects::*;
use query_structure::{prelude::ParentContainer, CompositeFieldRef};

/// Builds "where" argument.
pub(crate) fn where_argument<'a>(ctx: &'a QuerySchema, model: &Model) -> InputField<'a> {
    let where_object = filter_objects::where_object_type(ctx, model.into());

    input_field(args::WHERE.to_owned(), vec![InputType::object(where_object)], None).optional()
}

/// Builds "where" argument which input type is the where unique type of the input builder.
pub(crate) fn where_unique_argument(ctx: &QuerySchema, model: Model) -> InputField<'_> {
    let input_object_type = filter_objects::where_unique_object_type(ctx, model);
    input_field(args::WHERE.to_owned(), vec![InputType::object(input_object_type)], None)
}

/// Builds "relationLoadStrategy" argument, if the corresponding functionality is available.
pub(crate) fn relation_load_strategy_argument(ctx: &QuerySchema) -> Option<InputField<'_>> {
    enum_types::relation_load_strategy(ctx).map(|load_strategy_type| {
        input_field(
            args::RELATION_LOAD_STRATEGY,
            vec![InputType::Enum(load_strategy_type)],
            None,
        )
        .optional()
    })
}

/// Builds "where" (unique) and "relationLoadStrategy" arguments for the findUnique field.
pub(crate) fn find_unique_arguments(ctx: &QuerySchema, model: Model) -> Vec<InputField<'_>> {
    std::iter::once(where_unique_argument(ctx, model))
        .chain(relation_load_strategy_argument(ctx))
        .collect()
}

/// Builds "where" (unique) and "relationLoadStrategy" arguments intended for the delete field.
pub(crate) fn delete_one_arguments(ctx: &QuerySchema, model: Model) -> Vec<InputField<'_>> {
    std::iter::once(where_unique_argument(ctx, model))
        .chain(relation_load_strategy_argument(ctx))
        .collect()
}

/// Builds "where" (unique), "data" and "relationLoadStrategy" arguments intended for the update field.
pub(crate) fn update_one_arguments(ctx: &QuerySchema, model: Model) -> Vec<InputField<'_>> {
    let unique_arg = where_unique_argument(ctx, model.clone());
    let update_types = update_one_objects::update_one_input_types(ctx, model, None);

    let mut args = vec![input_field(args::DATA.to_owned(), update_types, None), unique_arg];

    args.extend(relation_load_strategy_argument(ctx));

    args
}

/// Builds "where" (unique), "create", "update" and "relationLoadStrategy" arguments intended for the upsert field.
pub(crate) fn upsert_arguments(ctx: &QuerySchema, model: Model) -> Vec<InputField<'_>> {
    let where_unique_arg = where_unique_argument(ctx, model.clone());
    let update_types = update_one_objects::update_one_input_types(ctx, model.clone(), None);
    let create_types = create_one::create_one_input_types(ctx, model, None);

    let mut args = vec![
        where_unique_arg,
        input_field(args::CREATE.to_owned(), create_types, None),
        input_field(args::UPDATE.to_owned(), update_types, None),
    ];

    args.extend(relation_load_strategy_argument(ctx));

    args
}

/// Builds "where" and "data" arguments intended for the update many field.
pub(crate) fn update_many_arguments(ctx: &QuerySchema, model: Model) -> Vec<InputField<'_>> {
    let update_many_types = update_many_objects::update_many_input_types(ctx, model.clone(), None);
    let where_arg = where_argument(ctx, &model);

    vec![input_field(args::DATA.to_owned(), update_many_types, None), where_arg]
}

/// Builds "where" argument intended for the delete many field.
pub(crate) fn delete_many_arguments(ctx: &QuerySchema, model: Model) -> Vec<InputField<'_>> {
    let where_arg = where_argument(ctx, &model);

    vec![
        where_arg,
        input_field(args::LIMIT, vec![InputType::int()], None).optional(),
    ]
}

/// Builds "many records where" arguments based on the given model and field.
pub(crate) fn many_records_output_field_arguments(ctx: &QuerySchema, field: ModelField) -> Vec<InputField<'_>> {
    match field {
        ModelField::Scalar(_) => vec![],

        // To-many relation.
        ModelField::Relation(rf) if rf.is_list() => relation_to_many_selection_arguments(ctx, rf.related_model()),

        // To-one optional relation.
        ModelField::Relation(rf) if !rf.is_required() => relation_to_one_selection_arguments(ctx, rf.related_model()),

        // To-one required relation.
        ModelField::Relation(_) => vec![],

        // To-many composite.
        ModelField::Composite(cf) if cf.is_list() => composite_selection_arguments(ctx, cf),

        // To-one composite.
        ModelField::Composite(_) => vec![],
    }
}

/// Builds "many records where" arguments for to-many relation selection sets.
pub(crate) fn relation_to_many_selection_arguments(ctx: &QuerySchema, model: Model) -> Vec<InputField<'_>> {
    ManyRecordsSelectionArgumentsBuilder::new(ctx, model)
        .include_distinct()
        .build()
}

/// Builds "many records where" arguments for to-many relation selection sets.
pub(crate) fn relation_to_one_selection_arguments(ctx: &QuerySchema, model: Model) -> Vec<InputField<'_>> {
    vec![where_argument(ctx, &model)]
}

/// Builds "many composite where" arguments for to-many composite selection sets.
pub(crate) fn composite_selection_arguments(_ctx: &QuerySchema, _cf: CompositeFieldRef) -> Vec<InputField<'_>> {
    vec![]
}

// Builds "orderBy" argument.
pub(crate) fn order_by_argument(
    ctx: &QuerySchema,
    container: ParentContainer,
    options: OrderByOptions,
) -> InputField<'_> {
    let order_object_type = InputType::object(order_by_objects::order_by_object_type(ctx, container, options));

    input_field(
        args::ORDER_BY.to_owned(),
        vec![InputType::list(order_object_type.clone()), order_object_type],
        None,
    )
    .optional()
}

pub(crate) fn group_by_arguments(ctx: &QuerySchema, model: Model) -> Vec<InputField<'_>> {
    let field_enum_type = InputType::Enum(model_field_enum(&model));
    let filter_object = InputType::object(filter_objects::scalar_filter_object_type(ctx, model.clone(), true));

    vec![
        where_argument(ctx, &model),
        order_by_argument(ctx, model.into(), OrderByOptions::new().with_aggregates()),
        input_field(
            args::BY,
            vec![InputType::list(field_enum_type.clone()), field_enum_type],
            None,
        ),
        input_field(args::HAVING, vec![filter_object], None).optional(),
        input_field(args::TAKE, vec![InputType::int()], None).optional(),
        input_field(args::SKIP, vec![InputType::int()], None).optional(),
    ]
}

pub(crate) struct ManyRecordsSelectionArgumentsBuilder<'a> {
    ctx: &'a QuerySchema,
    model: Model,
    include_distinct: bool,
    include_relation_load_strategy: bool,
}

impl<'a> ManyRecordsSelectionArgumentsBuilder<'a> {
    pub(crate) fn new(ctx: &'a QuerySchema, model: Model) -> Self {
        Self {
            ctx,
            model,
            include_distinct: false,
            include_relation_load_strategy: false,
        }
    }

    pub(crate) fn include_distinct(mut self) -> Self {
        self.include_distinct = true;
        self
    }

    pub(crate) fn include_relation_load_strategy(mut self) -> Self {
        self.include_relation_load_strategy = true;
        self
    }

    pub(crate) fn build(self) -> Vec<InputField<'a>> {
        let unique_input_type =
            InputType::object(filter_objects::where_unique_object_type(self.ctx, self.model.clone()));

        let order_by_options = OrderByOptions {
            include_relations: true,
            include_scalar_aggregations: false,
            include_full_text_search: self.ctx.can_full_text_search(),
        };

        let mut args = vec![
            where_argument(self.ctx, &self.model),
            order_by_argument(self.ctx, self.model.clone().into(), order_by_options),
            input_field(args::CURSOR, vec![unique_input_type], None).optional(),
            input_field(args::TAKE, vec![InputType::int()], None).optional(),
            input_field(args::SKIP, vec![InputType::int()], None).optional(),
        ];

        if self.include_distinct {
            let input_types = list_union_type(InputType::Enum(model_field_enum(&self.model)), true);
            args.push(input_field(args::DISTINCT, input_types, None).optional());
        }

        if self.include_relation_load_strategy {
            args.extend(relation_load_strategy_argument(self.ctx))
        }

        args
    }
}
