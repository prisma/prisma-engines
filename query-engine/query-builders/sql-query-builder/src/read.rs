use itertools::Itertools;
use quaint::ast::*;
use query_structure::*;
use schema::constants::aggregations::{
    UNDERSCORE_AVG, UNDERSCORE_COUNT, UNDERSCORE_MAX, UNDERSCORE_MIN, UNDERSCORE_SUM,
};

use crate::{
    context::Context,
    cursor_condition,
    filter::FilterBuilder,
    model_extensions::{AsColumn, AsColumns, AsTable},
    nested_aggregations,
    ordering::OrderByBuilder,
    sql_trace::SqlTraceComment,
};

pub trait SelectDefinition {
    fn into_select<'a>(
        self,
        _: &Model,
        virtual_selections: impl IntoIterator<Item = &'a VirtualSelection>,
        ctx: &Context<'_>,
    ) -> (Select<'static>, Vec<Expression<'static>>);
}

impl SelectDefinition for Filter {
    fn into_select<'a>(
        self,
        model: &Model,
        virtual_selections: impl IntoIterator<Item = &'a VirtualSelection>,
        ctx: &Context<'_>,
    ) -> (Select<'static>, Vec<Expression<'static>>) {
        let args = QueryArguments::from((model.clone(), self));
        args.into_select(model, virtual_selections, ctx)
    }
}

impl SelectDefinition for &Filter {
    fn into_select<'a>(
        self,
        model: &Model,
        virtual_selections: impl IntoIterator<Item = &'a VirtualSelection>,
        ctx: &Context<'_>,
    ) -> (Select<'static>, Vec<Expression<'static>>) {
        self.clone().into_select(model, virtual_selections, ctx)
    }
}

impl SelectDefinition for Select<'static> {
    fn into_select<'a>(
        self,
        _: &Model,
        _: impl IntoIterator<Item = &'a VirtualSelection>,
        _ctx: &Context<'_>,
    ) -> (Select<'static>, Vec<Expression<'static>>) {
        (self, vec![])
    }
}

impl SelectDefinition for QueryArguments {
    fn into_select<'a>(
        self,
        model: &Model,
        virtual_selections: impl IntoIterator<Item = &'a VirtualSelection>,
        ctx: &Context<'_>,
    ) -> (Select<'static>, Vec<Expression<'static>>) {
        let order_by_definitions = OrderByBuilder::default().build(&self, ctx);
        let cursor_condition = cursor_condition::build(&self, model, &order_by_definitions, ctx);
        let aggregation_joins = nested_aggregations::build(virtual_selections, ctx);

        let limit = if self.ignore_take { None } else { self.take_abs() };
        let skip = if self.ignore_skip { 0 } else { self.skip.unwrap_or(0) };

        let (filter, filter_joins) = self
            .filter
            .map(|f| FilterBuilder::with_top_level_joins().visit_filter(f, ctx))
            .unwrap_or((ConditionTree::NoCondition, None));

        let conditions = match (filter, cursor_condition) {
            (ConditionTree::NoCondition, cursor) => cursor,
            (filter, ConditionTree::NoCondition) => filter,
            (filter, cursor) => ConditionTree::and(filter, cursor),
        };

        // Add joins necessary to the ordering
        let joined_table = order_by_definitions
            .iter()
            .flat_map(|j| &j.joins)
            .fold(model.as_table(ctx), |acc, join| acc.join(join.clone().data));

        // Add joins necessary to the nested aggregations
        let joined_table = aggregation_joins
            .joins
            .into_iter()
            .fold(joined_table, |acc, join| acc.join(join.data));

        let joined_table = if let Some(filter_joins) = filter_joins {
            filter_joins
                .into_iter()
                .fold(joined_table, |acc, join| acc.join(join.data))
        } else {
            joined_table
        };

        let select_ast = Select::from_table(joined_table)
            .so_that(conditions)
            .offset(skip as usize)
            .add_traceparent(ctx.traceparent);

        let select_ast = order_by_definitions
            .iter()
            .fold(select_ast, |acc, o| acc.order_by(o.order_definition.clone()));

        let select_ast = if let Some(distinct) = self.distinct {
            let distinct_fields = ModelProjection::from(distinct)
                .as_columns(ctx)
                .map(Expression::from)
                .collect_vec();

            select_ast.distinct_on(distinct_fields)
        } else {
            select_ast
        };

        match limit {
            Some(limit) => (select_ast.limit(limit as usize), aggregation_joins.columns),
            None => (select_ast, aggregation_joins.columns),
        }
    }
}

pub fn get_records<'a, T>(
    model: &Model,
    columns: impl Iterator<Item = Column<'static>>,
    virtual_selections: impl IntoIterator<Item = &'a VirtualSelection>,
    query: T,
    ctx: &Context<'_>,
) -> Select<'static>
where
    T: SelectDefinition,
{
    let (select, additional_selection_set) = query.into_select(model, virtual_selections, ctx);
    let select = columns.fold(select, |acc, col| acc.column(col));

    let select = select.add_traceparent(ctx.traceparent);

    additional_selection_set
        .into_iter()
        .fold(select, |acc, col| acc.value(col))
}

/// Generates a query of the form:
/// ```sql
/// SELECT
///     COUNT(*),
///     SUM(`float`),
///     SUM(`int`),
///     AVG(`float`),
///     AVG(`int`),
///     MIN(`float`),
///     MIN(`int`),
///     MAX(`float`),
///     MAX(`int`)
/// FROM
///     (
///         SELECT
///             `Table`.`id`,
///             `Table`.`float`,
///             `Table`.`int`
///         FROM
///             `Table`
///         WHERE
///             1 = 1
///     ) AS `sub`;
/// ```
/// Important note: Do not use the AsColumn trait here as we need to construct column references that are relative,
/// not absolute - e.g. `SELECT "field" FROM (...)` NOT `SELECT "full"."path"."to"."field" FROM (...)`.
pub fn aggregate(
    model: &Model,
    selections: &[AggregationSelection],
    args: QueryArguments,
    alias: impl AliasGenerator,
    ctx: &Context<'_>,
) -> Select<'static> {
    let columns = extract_columns(model, selections, ctx);
    let sub_query = get_records(model, columns.into_iter(), &[], args, ctx);
    let sub_table = Table::from(sub_query).alias("sub");

    selections.iter().fold(
        Select::from_table(sub_table).add_traceparent(ctx.traceparent),
        |select, next_op| match next_op {
            AggregationSelection::Field(field) => select.column(
                alias
                    .apply(Column::from(field.db_name().to_owned()), field)
                    .set_is_enum(field.type_identifier().is_enum())
                    .set_is_selected(true),
            ),

            AggregationSelection::Count { all, fields } => {
                let select = fields.iter().fold(select, |select, next_field| {
                    select.value(
                        alias
                            .with_prefix(UNDERSCORE_COUNT)
                            .apply(count(Column::from(next_field.db_name().to_owned())), next_field),
                    )
                });

                if *all {
                    select.value(count(asterisk()).alias("_count"))
                } else {
                    select
                }
            }

            AggregationSelection::Average(fields) => fields.iter().fold(select, |select, next_field| {
                select.value(
                    alias
                        .with_prefix(UNDERSCORE_AVG)
                        .apply(avg(Column::from(next_field.db_name().to_owned())), next_field),
                )
            }),

            AggregationSelection::Sum(fields) => fields.iter().fold(select, |select, next_field| {
                select.value(
                    alias
                        .with_prefix(UNDERSCORE_SUM)
                        .apply(sum(Column::from(next_field.db_name().to_owned())), next_field),
                )
            }),

            AggregationSelection::Min(fields) => fields.iter().fold(select, |select, next_field| {
                select.value(
                    alias.with_prefix(UNDERSCORE_MIN).apply(
                        min(Column::from(next_field.db_name().to_owned())
                            .set_is_enum(next_field.type_identifier().is_enum())
                            .set_is_selected(true)),
                        next_field,
                    ),
                )
            }),

            AggregationSelection::Max(fields) => fields.iter().fold(select, |select, next_field| {
                select.value(
                    alias.apply(
                        max(Column::from(next_field.db_name().to_owned())
                            .set_is_enum(next_field.type_identifier().is_enum())
                            .set_is_selected(true)),
                        next_field,
                    ),
                )
            }),
        },
    )
}

pub fn group_by_aggregate(
    model: &Model,
    args: QueryArguments,
    selections: &[AggregationSelection],
    group_by: Vec<ScalarFieldRef>,
    having: Option<Filter>,
    alias: impl AliasGenerator,
    ctx: &Context<'_>,
) -> Select<'static> {
    let (base_query, _) = args.into_select(model, &[], ctx);
    let select_query = selections.iter().fold(base_query, |select, next_op| match next_op {
        AggregationSelection::Field(field) => {
            select.column(alias.apply(field.as_column(ctx), field).set_is_selected(true))
        }

        AggregationSelection::Count { all, fields } => {
            let select = fields.iter().fold(select, |select, next_field| {
                select.value(
                    alias
                        .with_prefix(UNDERSCORE_COUNT)
                        .apply(count(next_field.as_column(ctx)), next_field),
                )
            });

            if *all {
                select.value(count(asterisk()).alias("_count"))
            } else {
                select
            }
        }

        AggregationSelection::Average(fields) => fields.iter().fold(select, |select, next_field| {
            select.value(
                alias
                    .with_prefix(UNDERSCORE_AVG)
                    .apply(avg(next_field.as_column(ctx)), next_field),
            )
        }),

        AggregationSelection::Sum(fields) => fields.iter().fold(select, |select, next_field| {
            select.value(
                alias
                    .with_prefix(UNDERSCORE_SUM)
                    .apply(sum(next_field.as_column(ctx)), next_field),
            )
        }),

        AggregationSelection::Min(fields) => fields.iter().fold(select, |select, next_field| {
            select.value(
                alias
                    .with_prefix(UNDERSCORE_MIN)
                    .apply(min(next_field.as_column(ctx).set_is_selected(true)), next_field),
            )
        }),

        AggregationSelection::Max(fields) => fields.iter().fold(select, |select, next_field| {
            select.value(
                alias
                    .with_prefix(UNDERSCORE_MAX)
                    .apply(max(next_field.as_column(ctx).set_is_selected(true)), next_field),
            )
        }),
    });

    let grouped = group_by
        .into_iter()
        .fold(select_query.add_traceparent(ctx.traceparent), |query, field| {
            query.group_by(field.as_column(ctx))
        });

    match having {
        Some(filter) => {
            let cond = FilterBuilder::without_top_level_joins().visit_filter(filter, ctx);

            grouped.having(cond)
        }
        None => grouped,
    }
}

fn extract_columns(model: &Model, selections: &[AggregationSelection], ctx: &Context<'_>) -> Vec<Column<'static>> {
    let fields: Vec<_> = selections
        .iter()
        .flat_map(|selection| match selection {
            AggregationSelection::Field(field) => vec![field.clone()],
            AggregationSelection::Count { all: _, fields } => {
                if fields.is_empty() {
                    model
                        .primary_identifier()
                        .as_scalar_fields()
                        .expect("Primary identifier has non-scalar fields.")
                } else {
                    fields.clone()
                }
            }
            AggregationSelection::Average(fields) => fields.clone(),
            AggregationSelection::Sum(fields) => fields.clone(),
            AggregationSelection::Min(fields) => fields.clone(),
            AggregationSelection::Max(fields) => fields.clone(),
        })
        .unique_by(|field| field.db_name().to_owned())
        .collect();

    fields.as_columns(ctx).collect()
}

pub trait AliasGenerator {
    fn generate(&self, field: &ScalarField) -> Option<String>;

    /// Creates a new alias generator that prefixes all generated aliases with the given prefix.
    fn with_prefix<'a>(&'a self, prefix: &'a str) -> impl AliasGenerator + 'a
    where
        Self: Sized,
    {
        DotPrefixedAlias(prefix, self)
    }

    fn apply<'a, A: Aliasable<'a, Target = A>>(&self, expr: A, field: &ScalarField) -> A::Target {
        match self.generate(field) {
            Some(alias) => expr.alias(alias),
            None => expr,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct NoAlias;

impl AliasGenerator for NoAlias {
    fn generate(&self, _: &ScalarField) -> Option<String> {
        None
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct PrismaNameAlias;

impl AliasGenerator for PrismaNameAlias {
    fn generate(&self, field: &ScalarField) -> Option<String> {
        Some(field.name().to_owned())
    }
}

#[derive(Debug, Clone, Copy)]
struct DotPrefixedAlias<'a, Inner>(&'a str, &'a Inner);

impl<Inner> AliasGenerator for DotPrefixedAlias<'_, Inner>
where
    Inner: AliasGenerator,
{
    fn generate(&self, field: &ScalarField) -> Option<String> {
        let suffix = self.1.generate(field)?;
        Some(format!("{prefix}.{suffix}", prefix = self.0))
    }
}

/// Alias generator that uses the prisma name of the field.
pub fn alias_with_prisma_name() -> impl AliasGenerator {
    PrismaNameAlias
}

/// Alias generator that does not generate any aliases.
pub fn no_alias() -> impl AliasGenerator {
    NoAlias
}
