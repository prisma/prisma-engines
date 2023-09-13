use super::alias::*;
use crate::join_utils::{compute_one2m_join, AliasedJoin};
use crate::{model_extensions::*, Context};

use connector_interface::filter::*;
use prisma_models::prelude::*;
use quaint::ast::concat;
use quaint::ast::*;
use std::convert::TryInto;

pub(crate) trait FilterVisitorExt {
    fn visit_filter(&mut self, filter: Filter, ctx: &Context<'_>)
        -> (ConditionTree<'static>, Option<Vec<AliasedJoin>>);
    fn visit_relation_filter(
        &mut self,
        filter: RelationFilter,
        ctx: &Context<'_>,
    ) -> (ConditionTree<'static>, Option<Vec<AliasedJoin>>);
    fn visit_scalar_filter(&mut self, filter: ScalarFilter, ctx: &Context<'_>) -> ConditionTree<'static>;
    fn visit_scalar_list_filter(&mut self, filter: ScalarListFilter, ctx: &Context<'_>) -> ConditionTree<'static>;
    fn visit_one_relation_is_null_filter(
        &mut self,
        filter: OneRelationIsNullFilter,
        ctx: &Context<'_>,
    ) -> (ConditionTree<'static>, Option<Vec<AliasedJoin>>);
    fn visit_aggregation_filter(&mut self, filter: AggregationFilter, ctx: &Context<'_>) -> ConditionTree<'static>;
}

#[derive(Debug, Clone, Default)]
pub(crate) struct FilterVisitor {
    reverse: bool,
    last_alias: Option<Alias>,
    parent_alias: Option<Alias>,
    with_joins: bool,
}

impl FilterVisitor {
    pub fn with_joins() -> Self {
        Self {
            with_joins: true,
            ..Default::default()
        }
    }

    pub fn without_joins() -> Self {
        Self {
            with_joins: false,
            ..Default::default()
        }
    }

    fn invert_reverse(&mut self) -> &mut Self {
        self.reverse = !self.reverse;
        self
    }

    fn reverse(&self) -> bool {
        self.reverse
    }

    fn last_alias(&self) -> Option<Alias> {
        self.last_alias
    }

    fn next_alias(&mut self, mode: AliasMode) -> Alias {
        let next_alias = self.last_alias.unwrap_or_default().inc(mode);
        self.last_alias = Some(next_alias);

        next_alias
    }

    fn set_opt_last_alias(&mut self, alias: Option<Alias>) -> &mut Self {
        if let Some(alias) = alias {
            self.last_alias = Some(alias);
        }

        self
    }

    fn set_parent_alias(&mut self, alias: Alias) -> &mut Self {
        self.parent_alias = Some(alias);
        self
    }

    fn parent_alias(&self) -> Option<Alias> {
        self.parent_alias
    }

    fn visit_nested_filter(
        &mut self,
        filter: Filter,
        parent_alias: Alias,
        invert_reverse: bool,
        ctx: &Context<'_>,
    ) -> (ConditionTree<'static>, Option<Vec<AliasedJoin>>) {
        // We clone the visitor to avoid side-effects when building the nested joins.
        // For instance, we don't want the `parent_alias` to be set for the current visitor.
        let mut nested_visitor = self.clone();

        let nested_visitor = if invert_reverse {
            nested_visitor.invert_reverse()
        } else {
            &mut nested_visitor
        };

        // Sets the parent alias for the nested visitor.
        nested_visitor.set_parent_alias(parent_alias);

        let (cond, joins) = nested_visitor.visit_filter(filter, ctx);

        // Ensures the counter is updated after building the nested filter.
        self.set_opt_last_alias(nested_visitor.last_alias());

        (cond, joins)
    }

    fn visit_relation_filter_select(&mut self, filter: RelationFilter, ctx: &Context<'_>) -> Select<'static> {
        let alias = self.next_alias(AliasMode::Table);
        let condition = filter.condition;

        let table = filter.field.as_table(ctx);
        let selected_identifier: Vec<Column> = filter
            .field
            .identifier_columns(ctx)
            .map(|col| col.aliased_col(Some(alias), ctx))
            .collect();

        let join_columns: Vec<Column> = filter
            .field
            .join_columns(ctx)
            .map(|c| c.aliased_col(Some(alias), ctx))
            .collect();

        let related_table = filter.field.related_model().as_table(ctx);
        let related_join_columns: Vec<_> = ModelProjection::from(filter.field.related_field().linking_fields())
            .as_columns(ctx)
            .map(|col| col.aliased_col(Some(alias.flip(AliasMode::Join)), ctx))
            .collect();

        let (nested_conditions, joins) =
            self.visit_nested_filter(*filter.nested_filter, alias.flip(AliasMode::Join), false, ctx);

        let conditions = nested_conditions.invert_if(condition.invert_of_subselect());

        let conditions = selected_identifier
            .clone()
            .into_iter()
            .fold(conditions, |acc, column| acc.and(column.is_not_null()));

        let join = related_table
            .alias(alias.to_string(Some(AliasMode::Join)))
            .on(Row::from(related_join_columns).equals(Row::from(join_columns)));

        let select = Select::from_table(table.alias(alias.to_string(Some(AliasMode::Table))))
            .columns(selected_identifier)
            .inner_join(join)
            .so_that(conditions);

        if let Some(joins) = joins {
            joins.into_iter().fold(select, |acc, join| acc.join(join.data))
        } else {
            select
        }
    }
}

impl FilterVisitorExt for FilterVisitor {
    fn visit_filter(
        &mut self,
        filter: Filter,
        ctx: &Context<'_>,
    ) -> (ConditionTree<'static>, Option<Vec<AliasedJoin>>) {
        match filter {
            Filter::And(mut filters) => match filters.len() {
                n if n == 0 => (ConditionTree::NoCondition, None),
                n if n == 1 => self.visit_filter(filters.pop().unwrap(), ctx),
                _ => {
                    let mut exprs = vec![];
                    let mut joins = vec![];

                    for filter in filters {
                        let (conditions, nested_joins) = self.visit_filter(filter, ctx);

                        exprs.push(Expression::from(conditions));

                        if let Some(nested_joins) = nested_joins {
                            joins.extend(nested_joins);
                        }
                    }

                    (ConditionTree::And(exprs), Some(joins))
                }
            },
            Filter::Or(mut filters) => match filters.len() {
                n if n == 0 => (ConditionTree::NegativeCondition, None),
                n if n == 1 => self.visit_filter(filters.pop().unwrap(), ctx),
                _ => {
                    let mut exprs = vec![];
                    let mut joins = vec![];

                    for filter in filters {
                        let (conditions, nested_joins) = self.visit_filter(filter, ctx);

                        exprs.push(Expression::from(conditions));

                        if let Some(nested_joins) = nested_joins {
                            joins.extend(nested_joins);
                        }
                    }

                    (ConditionTree::Or(exprs), Some(joins))
                }
            },
            Filter::Not(mut filters) => match filters.len() {
                n if n == 0 => (ConditionTree::NoCondition, None),
                n if n == 1 => {
                    self.invert_reverse();
                    let (cond, joins) = self.visit_filter(filters.pop().unwrap(), ctx);

                    (cond.not(), joins)
                }
                _ => {
                    let mut exprs = vec![];
                    let mut joins = vec![];

                    self.invert_reverse();

                    for filter in filters {
                        let (conditions, nested_joins) = self.visit_filter(filter, ctx);
                        let inverted_condition = conditions.not();

                        exprs.push(Expression::from(inverted_condition));

                        if let Some(nested_joins) = nested_joins {
                            joins.extend(nested_joins);
                        }
                    }

                    (ConditionTree::And(exprs), Some(joins))
                }
            },
            Filter::Scalar(filter) => (self.visit_scalar_filter(filter, ctx), None),
            Filter::OneRelationIsNull(filter) => self.visit_one_relation_is_null_filter(filter, ctx),
            Filter::Relation(filter) => self.visit_relation_filter(filter, ctx),
            Filter::BoolFilter(b) => {
                if b {
                    (ConditionTree::NoCondition, None)
                } else {
                    (ConditionTree::NegativeCondition, None)
                }
            }
            Filter::Aggregation(filter) => (self.visit_aggregation_filter(filter, ctx), None),
            Filter::ScalarList(filter) => (self.visit_scalar_list_filter(filter, ctx), None),
            Filter::Empty => (ConditionTree::NoCondition, None),
            Filter::Composite(_) => unimplemented!("SQL connectors do not support composites yet."),
        }
    }

    fn visit_scalar_filter(&mut self, filter: ScalarFilter, ctx: &Context<'_>) -> ConditionTree<'static> {
        match filter.condition {
            ScalarCondition::Search(_, _) | ScalarCondition::NotSearch(_, _) => {
                let mut projections = match filter.condition.clone() {
                    ScalarCondition::Search(_, proj) => proj,
                    ScalarCondition::NotSearch(_, proj) => proj,
                    _ => unreachable!(),
                };

                projections.push(filter.projection);

                let columns: Vec<Column> = projections
                    .into_iter()
                    .map(|p| match p {
                        ScalarProjection::Single(field) => field.aliased_col(self.parent_alias(), ctx),
                        ScalarProjection::Compound(_) => {
                            unreachable!("Full-text search does not support compound fields")
                        }
                    })
                    .collect();

                let comparable: Expression = text_search(columns.as_slice()).into();

                convert_scalar_filter(
                    comparable,
                    filter.condition,
                    self.reverse(),
                    filter.mode,
                    &[],
                    self.parent_alias(),
                    false,
                    ctx,
                )
            }
            _ => scalar_filter_aliased_cond(filter, self.parent_alias(), self.reverse(), ctx),
        }
    }

    fn visit_relation_filter(
        &mut self,
        filter: RelationFilter,
        ctx: &Context<'_>,
    ) -> (ConditionTree<'static>, Option<Vec<AliasedJoin>>) {
        let parent_alias = self.parent_alias().map(|a| a.to_string(Some(AliasMode::Join)));

        match &filter.condition {
            RelationCondition::NoRelatedRecord if self.with_joins && !filter.field.is_list() => {
                let alias = self.next_alias(AliasMode::Join);

                let join = compute_one2m_join(
                    &filter.field,
                    alias.to_string(None).as_str(),
                    parent_alias.as_deref(),
                    ctx,
                );

                let mut output_joins = vec![join];

                let (conditions, nested_joins) = self.visit_nested_filter(*filter.nested_filter, alias, true, ctx);

                if let Some(nested_joins) = nested_joins {
                    output_joins.extend(nested_joins);
                }

                (conditions.not(), Some(output_joins))
            }
            RelationCondition::ToOneRelatedRecord if self.with_joins && !filter.field.is_list() => {
                let alias = self.next_alias(AliasMode::Join);

                let linking_fields: Vec<_> = ModelProjection::from(filter.field.model().primary_identifier())
                    .as_columns(ctx)
                    .map(|c| c.aliased_col(Some(alias), ctx))
                    .collect();
                let no_id_null_filter = Row::from(linking_fields).is_not_null();

                let join = compute_one2m_join(
                    &filter.field,
                    alias.to_string(None).as_str(),
                    parent_alias.as_deref(),
                    ctx,
                );
                let mut output_joins = vec![join];

                let (conditions, nested_joins) = self.visit_nested_filter(*filter.nested_filter, alias, false, ctx);

                if let Some(nested_joins) = nested_joins {
                    output_joins.extend(nested_joins);
                };

                (conditions.and(no_id_null_filter), Some(output_joins))
            }

            _ => {
                let ids = ModelProjection::from(filter.field.model().primary_identifier()).as_columns(ctx);
                let columns: Vec<Column<'static>> = ids.map(|col| col.aliased_col(self.parent_alias(), ctx)).collect();

                let condition = filter.condition;
                let sub_select = self.visit_relation_filter_select(filter, ctx);

                let comparison = match condition {
                    RelationCondition::AtLeastOneRelatedRecord => Row::from(columns).in_selection(sub_select),
                    RelationCondition::EveryRelatedRecord => Row::from(columns).not_in_selection(sub_select),
                    RelationCondition::NoRelatedRecord => Row::from(columns).not_in_selection(sub_select),
                    RelationCondition::ToOneRelatedRecord => Row::from(columns).in_selection(sub_select),
                };

                (comparison.into(), None)
            }
        }
    }

    fn visit_one_relation_is_null_filter(
        &mut self,
        filter: OneRelationIsNullFilter,
        ctx: &Context<'_>,
    ) -> (ConditionTree<'static>, Option<Vec<AliasedJoin>>) {
        let parent_alias = self.parent_alias();
        let parent_alias_string = parent_alias.as_ref().map(|a| a.to_string(None));

        if filter.field.is_inlined_on_enclosing_model() {
            let columns: Vec<_> = filter
                .field
                .as_columns(ctx)
                .map(|c| c.opt_table(parent_alias_string.clone()))
                .collect();
            let condition = Row::from(columns).is_null();

            (ConditionTree::single(condition), None)
        } else {
            if self.with_joins {
                let alias = self.next_alias(AliasMode::Join);

                let id_columns: Vec<_> = ModelProjection::from(filter.field.model().primary_identifier())
                    .as_columns(ctx)
                    .map(|c| c.aliased_col(Some(alias), ctx))
                    .collect();

                let join = compute_one2m_join(
                    &filter.field,
                    alias.to_string(None).as_str(),
                    parent_alias_string.as_deref(),
                    ctx,
                );

                (ConditionTree::single(Row::from(id_columns).is_null()), Some(vec![join]))
            } else {
                let relation = filter.field.relation();
                let table = relation.as_table(ctx);
                let relation_table = match parent_alias {
                    Some(ref alias) => table.alias(alias.to_string(None)),
                    None => table,
                };

                let columns_not_null =
                    filter
                        .field
                        .related_field()
                        .as_columns(ctx)
                        .fold(ConditionTree::NoCondition, |acc, column| {
                            let column_is_not_null = column.opt_table(parent_alias_string.clone()).is_not_null();

                            match acc {
                                ConditionTree::NoCondition => column_is_not_null.into(),
                                cond => cond.and(column_is_not_null),
                            }
                        });

                // If the table is aliased, we need to use that alias in the SELECT too
                // eg: SELECT <alias>.x FROM table AS <alias>
                let columns: Vec<_> = filter
                    .field
                    .related_field()
                    .scalar_fields()
                    .iter()
                    .map(|f| f.as_column(ctx).opt_table(parent_alias_string.clone()))
                    .collect();

                let sub_select = Select::from_table(relation_table)
                    .columns(columns)
                    .and_where(columns_not_null);

                let id_columns: Vec<Column<'static>> = ModelProjection::from(filter.field.linking_fields())
                    .as_columns(ctx)
                    .map(|c| c.opt_table(parent_alias_string.clone()))
                    .collect();

                (
                    ConditionTree::single(Row::from(id_columns).not_in_selection(sub_select)),
                    None,
                )
            }
        }
    }

    fn visit_aggregation_filter(&mut self, filter: AggregationFilter, ctx: &Context<'_>) -> ConditionTree<'static> {
        let alias = self.parent_alias();
        let reverse = self.reverse();

        match filter {
            AggregationFilter::Count(filter) => aggregate_conditions(*filter, alias, reverse, |x| count(x).into(), ctx),
            AggregationFilter::Average(filter) => aggregate_conditions(*filter, alias, reverse, |x| avg(x).into(), ctx),
            AggregationFilter::Sum(filter) => aggregate_conditions(*filter, alias, reverse, |x| sum(x).into(), ctx),
            AggregationFilter::Min(filter) => aggregate_conditions(*filter, alias, reverse, |x| min(x).into(), ctx),
            AggregationFilter::Max(filter) => aggregate_conditions(*filter, alias, reverse, |x| max(x).into(), ctx),
        }
    }

    fn visit_scalar_list_filter(&mut self, filter: ScalarListFilter, ctx: &Context<'_>) -> ConditionTree<'static> {
        let comparable: Expression = filter.field.aliased_col(self.parent_alias(), ctx).into();

        convert_scalar_list_filter(comparable, filter.condition, &filter.field, self.parent_alias(), ctx)
    }
}

fn scalar_filter_aliased_cond(
    sf: ScalarFilter,
    alias: Option<Alias>,
    reverse: bool,
    ctx: &Context<'_>,
) -> ConditionTree<'static> {
    match sf.projection {
        ScalarProjection::Single(field) => {
            let comparable: Expression = field.aliased_col(alias, ctx).into();

            convert_scalar_filter(comparable, sf.condition, reverse, sf.mode, &[field], alias, false, ctx)
        }
        ScalarProjection::Compound(fields) => {
            let columns: Vec<Column<'static>> = fields
                .clone()
                .into_iter()
                .map(|field| field.aliased_col(alias, ctx))
                .collect();

            convert_scalar_filter(
                Row::from(columns).into(),
                sf.condition,
                reverse,
                sf.mode,
                &fields,
                alias,
                false,
                ctx,
            )
        }
    }
}

fn convert_scalar_list_filter(
    comparable: Expression<'static>,
    cond: ScalarListCondition,
    field: &ScalarFieldRef,
    alias: Option<Alias>,
    ctx: &Context<'_>,
) -> ConditionTree<'static> {
    let condition = match cond {
        ScalarListCondition::Contains(ConditionValue::Value(val)) => {
            comparable.compare_raw("@>", convert_list_pv(field, vec![val]))
        }
        ScalarListCondition::Contains(ConditionValue::FieldRef(field_ref)) => {
            let field_ref_expr: Expression = field_ref.aliased_col(alias, ctx).into();

            // This code path is only reachable for connectors with `ScalarLists` capability
            field_ref_expr.equals(comparable.any())
        }
        ScalarListCondition::ContainsEvery(ConditionListValue::List(vals)) => {
            comparable.compare_raw("@>", convert_list_pv(field, vals))
        }
        ScalarListCondition::ContainsEvery(ConditionListValue::FieldRef(field_ref)) => {
            comparable.compare_raw("@>", field_ref.aliased_col(alias, ctx))
        }
        ScalarListCondition::ContainsSome(ConditionListValue::List(vals)) => {
            comparable.compare_raw("&&", convert_list_pv(field, vals))
        }
        ScalarListCondition::ContainsSome(ConditionListValue::FieldRef(field_ref)) => {
            comparable.compare_raw("&&", field_ref.aliased_col(alias, ctx))
        }
        ScalarListCondition::IsEmpty(true) => comparable.compare_raw("=", Value::Array(Some(vec![])).raw()),
        ScalarListCondition::IsEmpty(false) => comparable.compare_raw("<>", Value::Array(Some(vec![])).raw()),
    };

    ConditionTree::single(condition)
}

fn aggregate_conditions<T>(
    filter: Filter,
    alias: Option<Alias>,
    reverse: bool,
    field_transformer: T,
    ctx: &Context<'_>,
) -> ConditionTree<'static>
where
    T: Fn(Column) -> Expression,
{
    let sf = match filter {
        Filter::Scalar(sf) => sf,
        _ => unimplemented!(),
    };

    match sf.projection {
        ScalarProjection::Compound(_) => {
            unimplemented!("Compound aggregate projections are unsupported.")
        }
        ScalarProjection::Single(field) => {
            let comparable: Expression = field_transformer(field.aliased_col(alias, ctx));

            convert_scalar_filter(comparable, sf.condition, reverse, sf.mode, &[field], alias, true, ctx)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn convert_scalar_filter(
    comparable: Expression<'static>,
    cond: ScalarCondition,
    reverse: bool,
    mode: QueryMode,
    fields: &[ScalarFieldRef],
    alias: Option<Alias>,
    is_parent_aggregation: bool,
    ctx: &Context<'_>,
) -> ConditionTree<'static> {
    match cond {
        ScalarCondition::JsonCompare(json_compare) => convert_json_filter(
            comparable,
            json_compare,
            reverse,
            fields.first().unwrap(),
            mode,
            alias,
            ctx,
        ),
        _ => match mode {
            QueryMode::Default => default_scalar_filter(comparable, cond, fields, alias, ctx),
            QueryMode::Insensitive => {
                insensitive_scalar_filter(comparable, cond, fields, alias, is_parent_aggregation, ctx)
            }
        },
    }
}

fn convert_json_filter(
    comparable: Expression<'static>,
    json_condition: JsonCondition,
    reverse: bool,
    field: &ScalarFieldRef,
    query_mode: QueryMode,
    alias: Option<Alias>,
    ctx: &Context<'_>,
) -> ConditionTree<'static> {
    let JsonCondition {
        path,
        condition,
        target_type,
    } = json_condition;
    let (expr_json, expr_string): (Expression, Expression) = match path {
        Some(JsonFilterPath::String(path)) => (
            json_extract(comparable.clone(), JsonPath::string(path.clone()), false).into(),
            json_extract(comparable, JsonPath::string(path), true).into(),
        ),
        Some(JsonFilterPath::Array(path)) => (
            json_extract(comparable.clone(), JsonPath::array(path.clone()), false).into(),
            json_extract(comparable, JsonPath::array(path), true).into(),
        ),
        _ => (comparable.clone(), comparable),
    };

    let condition: Expression = match *condition {
        ScalarCondition::Contains(value) => {
            (expr_json, expr_string).json_contains(field, value, target_type.unwrap(), reverse, alias, ctx)
        }
        ScalarCondition::StartsWith(value) => {
            (expr_json, expr_string).json_starts_with(field, value, target_type.unwrap(), reverse, alias, ctx)
        }
        ScalarCondition::EndsWith(value) => {
            (expr_json, expr_string).json_ends_with(field, value, target_type.unwrap(), reverse, alias, ctx)
        }
        ScalarCondition::GreaterThan(value) => {
            let gt = expr_json
                .clone()
                .greater_than(convert_value(field, value.clone(), alias, ctx));

            with_json_type_filter(gt, expr_json, value, alias, reverse, ctx)
        }
        ScalarCondition::GreaterThanOrEquals(value) => {
            let gte = expr_json
                .clone()
                .greater_than_or_equals(convert_value(field, value.clone(), alias, ctx));

            with_json_type_filter(gte, expr_json, value, alias, reverse, ctx)
        }
        ScalarCondition::LessThan(value) => {
            let lt = expr_json
                .clone()
                .less_than(convert_value(field, value.clone(), alias, ctx));

            with_json_type_filter(lt, expr_json, value, alias, reverse, ctx)
        }
        ScalarCondition::LessThanOrEquals(value) => {
            let lte = expr_json
                .clone()
                .less_than_or_equals(convert_value(field, value.clone(), alias, ctx));

            with_json_type_filter(lte, expr_json, value, alias, reverse, ctx)
        }
        // Those conditions are unreachable because json filters are not accessible via the lowercase `not`.
        // They can only be inverted via the uppercase `NOT`, which doesn't invert filters but adds a Filter::Not().
        ScalarCondition::NotContains(_) => unreachable!(),
        ScalarCondition::NotStartsWith(_) => unreachable!(),
        ScalarCondition::NotEndsWith(_) => unreachable!(),
        cond => {
            return convert_scalar_filter(
                expr_json,
                cond,
                reverse,
                query_mode,
                &[field.clone()],
                alias,
                false,
                ctx,
            );
        }
    };

    ConditionTree::single(condition)
}

fn with_json_type_filter(
    comparable: Compare<'static>,
    expr_json: Expression<'static>,
    value: ConditionValue,
    alias: Option<Alias>,
    reverse: bool,
    ctx: &Context<'_>,
) -> Expression<'static> {
    match value {
        ConditionValue::Value(pv) => match pv {
            PrismaValue::Json(json) => {
                let json: serde_json::Value = serde_json::from_str(json.as_str()).unwrap();

                match json {
                    serde_json::Value::String(_) if reverse => {
                        comparable.or(expr_json.json_type_not_equals(JsonType::String)).into()
                    }
                    serde_json::Value::String(_) => comparable.and(expr_json.json_type_equals(JsonType::String)).into(),
                    serde_json::Value::Number(_) if reverse => {
                        comparable.or(expr_json.json_type_not_equals(JsonType::Number)).into()
                    }
                    serde_json::Value::Number(_) => comparable.and(expr_json.json_type_equals(JsonType::Number)).into(),
                    v => panic!("JSON target types only accept strings or numbers, found: {v}"),
                }
            }
            _ => unreachable!(),
        },
        ConditionValue::FieldRef(field_ref) if reverse => comparable
            .or(expr_json.json_type_not_equals(field_ref.aliased_col(alias, ctx)))
            .into(),
        ConditionValue::FieldRef(field_ref) => comparable
            .and(expr_json.json_type_equals(field_ref.aliased_col(alias, ctx)))
            .into(),
    }
}

fn default_scalar_filter(
    comparable: Expression<'static>,
    cond: ScalarCondition,
    fields: &[ScalarFieldRef],
    alias: Option<Alias>,
    ctx: &Context<'_>,
) -> ConditionTree<'static> {
    let condition = match cond {
        ScalarCondition::Equals(ConditionValue::Value(PrismaValue::Null)) => comparable.is_null(),
        ScalarCondition::NotEquals(ConditionValue::Value(PrismaValue::Null)) => comparable.is_not_null(),
        ScalarCondition::Equals(value) => comparable.equals(convert_first_value(fields, value, alias, ctx)),
        ScalarCondition::NotEquals(value) => comparable.not_equals(convert_first_value(fields, value, alias, ctx)),
        ScalarCondition::Contains(value) => match value {
            ConditionValue::Value(value) => comparable.like(format!("%{value}%")),
            ConditionValue::FieldRef(field_ref) => comparable.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                Value::text("%").raw().into(),
                field_ref.aliased_col(alias, ctx).into(),
                Value::text("%").raw().into(),
            ])),
        },
        ScalarCondition::NotContains(value) => match value {
            ConditionValue::Value(value) => comparable.not_like(format!("%{value}%")),
            ConditionValue::FieldRef(field_ref) => {
                comparable.not_like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                    Value::text("%").raw().into(),
                    field_ref.aliased_col(alias, ctx).into(),
                    Value::text("%").raw().into(),
                ]))
            }
        },
        ScalarCondition::StartsWith(value) => match value {
            ConditionValue::Value(value) => comparable.like(format!("{value}%")),
            ConditionValue::FieldRef(field_ref) => comparable.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                field_ref.aliased_col(alias, ctx).into(),
                Value::text("%").raw().into(),
            ])),
        },
        ScalarCondition::NotStartsWith(value) => match value {
            ConditionValue::Value(value) => comparable.not_like(format!("{value}%")),
            ConditionValue::FieldRef(field_ref) => {
                comparable.not_like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                    field_ref.aliased_col(alias, ctx).into(),
                    Value::text("%").raw().into(),
                ]))
            }
        },
        ScalarCondition::EndsWith(value) => match value {
            ConditionValue::Value(value) => comparable.like(format!("%{value}")),
            ConditionValue::FieldRef(field_ref) => comparable.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                Value::text("%").raw().into(),
                field_ref.aliased_col(alias, ctx).into(),
            ])),
        },
        ScalarCondition::NotEndsWith(value) => match value {
            ConditionValue::Value(value) => comparable.not_like(format!("%{value}")),
            ConditionValue::FieldRef(field_ref) => {
                comparable.not_like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                    Value::text("%").raw().into(),
                    field_ref.aliased_col(alias, ctx).into(),
                ]))
            }
        },
        ScalarCondition::LessThan(value) => comparable.less_than(convert_first_value(fields, value, alias, ctx)),
        ScalarCondition::LessThanOrEquals(value) => {
            comparable.less_than_or_equals(convert_first_value(fields, value, alias, ctx))
        }
        ScalarCondition::GreaterThan(value) => comparable.greater_than(convert_first_value(fields, value, alias, ctx)),
        ScalarCondition::GreaterThanOrEquals(value) => {
            comparable.greater_than_or_equals(convert_first_value(fields, value, alias, ctx))
        }
        ScalarCondition::In(ConditionListValue::List(values)) => match values.split_first() {
            Some((PrismaValue::List(_), _)) => {
                let mut sql_values = Values::with_capacity(values.len());

                for pv in values {
                    let list_value = convert_pvs(fields, pv.into_list().unwrap());
                    sql_values.push(list_value);
                }

                comparable.in_selection(sql_values)
            }
            _ => comparable.in_selection(convert_pvs(fields, values)),
        },
        ScalarCondition::In(ConditionListValue::FieldRef(field_ref)) => {
            // This code path is only reachable for connectors with `ScalarLists` capability
            comparable.equals(Expression::from(field_ref.aliased_col(alias, ctx)).any())
        }
        ScalarCondition::NotIn(ConditionListValue::List(values)) => match values.split_first() {
            Some((PrismaValue::List(_), _)) => {
                let mut sql_values = Values::with_capacity(values.len());

                for pv in values {
                    let list_value = convert_pvs(fields, pv.into_list().unwrap());
                    sql_values.push(list_value);
                }

                comparable.not_in_selection(sql_values)
            }
            _ => comparable.not_in_selection(convert_pvs(fields, values)),
        },
        ScalarCondition::NotIn(ConditionListValue::FieldRef(field_ref)) => {
            // This code path is only reachable for connectors with `ScalarLists` capability
            comparable.not_equals(Expression::from(field_ref.aliased_col(alias, ctx)).all())
        }
        ScalarCondition::Search(value, _) => {
            let query: String = value
                .into_value()
                .unwrap()
                .try_into()
                .unwrap_or_else(|err: ConversionFailure| panic!("{}", err));

            comparable.matches(query)
        }
        ScalarCondition::NotSearch(value, _) => {
            let query: String = value
                .into_value()
                .unwrap()
                .try_into()
                .unwrap_or_else(|err: ConversionFailure| panic!("{}", err));

            comparable.not_matches(query)
        }
        ScalarCondition::JsonCompare(_) => unreachable!(),
        ScalarCondition::IsSet(_) => unreachable!(),
    };

    ConditionTree::single(condition)
}

fn insensitive_scalar_filter(
    comparable: Expression<'static>,
    cond: ScalarCondition,
    fields: &[ScalarFieldRef],
    alias: Option<Alias>,
    is_parent_aggregation: bool,
    ctx: &Context<'_>,
) -> ConditionTree<'static> {
    // Current workaround: We assume we can use ILIKE when we see `mode: insensitive`, because postgres is the only DB that has
    // insensitive. We need a connector context for filter building that is unexpectedly complicated to integrate.
    let condition = match cond {
        ScalarCondition::Equals(ConditionValue::Value(PrismaValue::Null)) => comparable.is_null(),
        ScalarCondition::Equals(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("ILIKE", format!("{value}")),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw("ILIKE", field_ref.aliased_col(alias, ctx)),
        },
        ScalarCondition::NotEquals(ConditionValue::Value(PrismaValue::Null)) => comparable.is_not_null(),
        ScalarCondition::NotEquals(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("NOT ILIKE", format!("{value}")),
            ConditionValue::FieldRef(field_ref) => {
                comparable.compare_raw("NOT ILIKE", field_ref.aliased_col(alias, ctx))
            }
        },
        ScalarCondition::Contains(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("ILIKE", format!("%{value}%")),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw(
                "ILIKE",
                concat::<'_, Expression<'_>>(vec![
                    Value::text("%").into(),
                    field_ref.aliased_col(alias, ctx).into(),
                    Value::text("%").into(),
                ]),
            ),
        },
        ScalarCondition::NotContains(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("NOT ILIKE", format!("%{value}%")),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw(
                "NOT ILIKE",
                concat::<'_, Expression<'_>>(vec![
                    Value::text("%").into(),
                    field_ref.aliased_col(alias, ctx).into(),
                    Value::text("%").into(),
                ]),
            ),
        },
        ScalarCondition::StartsWith(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("ILIKE", format!("{value}%")),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw(
                "ILIKE",
                concat::<'_, Expression<'_>>(vec![field_ref.aliased_col(alias, ctx).into(), Value::text("%").into()]),
            ),
        },
        ScalarCondition::NotStartsWith(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("NOT ILIKE", format!("{value}%")),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw(
                "NOT ILIKE",
                concat::<'_, Expression<'_>>(vec![field_ref.aliased_col(alias, ctx).into(), Value::text("%").into()]),
            ),
        },
        ScalarCondition::EndsWith(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("ILIKE", format!("%{value}")),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw(
                "ILIKE",
                concat::<'_, Expression<'_>>(vec![Value::text("%").into(), field_ref.aliased_col(alias, ctx).into()]),
            ),
        },
        ScalarCondition::NotEndsWith(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("NOT ILIKE", format!("%{value}")),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw(
                "NOT ILIKE",
                concat::<'_, Expression<'_>>(vec![Value::text("%").into(), field_ref.aliased_col(alias, ctx).into()]),
            ),
        },
        ScalarCondition::LessThan(value) => {
            let comparable: Expression = lower_if(comparable, !is_parent_aggregation);

            comparable.less_than(lower(convert_first_value(fields, value, alias, ctx)))
        }
        ScalarCondition::LessThanOrEquals(value) => {
            let comparable: Expression = lower_if(comparable, !is_parent_aggregation);

            comparable.less_than_or_equals(lower(convert_first_value(fields, value, alias, ctx)))
        }
        ScalarCondition::GreaterThan(value) => {
            let comparable: Expression = lower_if(comparable, !is_parent_aggregation);

            comparable.greater_than(lower(convert_first_value(fields, value, alias, ctx)))
        }
        ScalarCondition::GreaterThanOrEquals(value) => {
            let comparable: Expression = lower_if(comparable, !is_parent_aggregation);

            comparable.greater_than_or_equals(lower(convert_first_value(fields, value, alias, ctx)))
        }
        ScalarCondition::In(ConditionListValue::List(values)) => match values.split_first() {
            Some((PrismaValue::List(_), _)) => {
                let mut sql_values = Values::with_capacity(values.len());

                for pv in values {
                    let list_value = convert_pvs(fields, pv.into_list().unwrap());
                    sql_values.push(list_value);
                }

                let comparable: Expression = lower_if(comparable, !is_parent_aggregation);

                comparable.in_selection(sql_values)
            }
            _ => {
                let comparable: Expression = lower_if(comparable, !is_parent_aggregation);

                comparable.in_selection(
                    values
                        .into_iter()
                        .map(|value| {
                            let val: Expression = lower(convert_first_value(fields, value, alias, ctx)).into();
                            val
                        })
                        .collect::<Vec<_>>(),
                )
            }
        },
        ScalarCondition::In(ConditionListValue::FieldRef(field_ref)) => {
            // This code path is only reachable for connectors with `ScalarLists` capability
            comparable.compare_raw("ILIKE", Expression::from(field_ref.aliased_col(alias, ctx)).any())
        }
        ScalarCondition::NotIn(ConditionListValue::List(values)) => match values.split_first() {
            Some((PrismaValue::List(_), _)) => {
                let mut sql_values = Values::with_capacity(values.len());

                for pv in values {
                    let list_value = convert_pvs(fields, pv.into_list().unwrap());
                    sql_values.push(list_value);
                }

                let comparable: Expression = lower(comparable).into();

                comparable.not_in_selection(sql_values)
            }
            _ => {
                let comparable: Expression = lower(comparable).into();

                comparable.not_in_selection(
                    values
                        .into_iter()
                        .map(|value| {
                            let val: Expression = lower(convert_first_value(fields, value, alias, ctx)).into();
                            val
                        })
                        .collect::<Vec<_>>(),
                )
            }
        },
        ScalarCondition::NotIn(ConditionListValue::FieldRef(field_ref)) => {
            // This code path is only reachable for connectors with `ScalarLists` capability
            comparable.compare_raw("NOT ILIKE", Expression::from(field_ref.aliased_col(alias, ctx)).all())
        }
        ScalarCondition::Search(value, _) => {
            let query: String = value
                .into_value()
                .unwrap()
                .try_into()
                .unwrap_or_else(|err: ConversionFailure| panic!("{}", err));

            comparable.matches(query)
        }
        ScalarCondition::NotSearch(value, _) => {
            let query: String = value
                .into_value()
                .unwrap()
                .try_into()
                .unwrap_or_else(|err: ConversionFailure| panic!("{}", err));

            comparable.not_matches(query)
        }
        ScalarCondition::JsonCompare(_) => unreachable!(),
        ScalarCondition::IsSet(_) => unreachable!(),
    };

    ConditionTree::single(condition)
}

fn lower_if(expr: Expression<'_>, cond: bool) -> Expression<'_> {
    if cond {
        lower(expr).into()
    } else {
        expr
    }
}

fn convert_value<'a>(
    field: &ScalarFieldRef,
    value: impl Into<ConditionValue>,
    alias: Option<Alias>,
    ctx: &Context<'_>,
) -> Expression<'a> {
    match value.into() {
        ConditionValue::Value(pv) => convert_pv(field, pv),
        ConditionValue::FieldRef(field_ref) => field_ref.aliased_col(alias, ctx).into(),
    }
}

fn convert_first_value<'a>(
    fields: &[ScalarFieldRef],
    value: impl Into<ConditionValue>,
    alias: Option<Alias>,
    ctx: &Context<'_>,
) -> Expression<'a> {
    match value.into() {
        ConditionValue::Value(pv) => convert_pv(fields.first().unwrap(), pv),
        ConditionValue::FieldRef(field_ref) => field_ref.aliased_col(alias, ctx).into(),
    }
}

fn convert_pv<'a>(field: &ScalarFieldRef, pv: PrismaValue) -> Expression<'a> {
    field.value(pv).into()
}

fn convert_list_pv<'a>(field: &ScalarFieldRef, values: Vec<PrismaValue>) -> Expression<'a> {
    Value::Array(Some(values.into_iter().map(|val| field.value(val)).collect())).into()
}

fn convert_pvs<'a>(fields: &[ScalarFieldRef], values: Vec<PrismaValue>) -> Vec<Value<'a>> {
    if fields.len() == values.len() {
        fields
            .iter()
            .zip(values)
            .map(|(field, value)| field.value(value))
            .collect()
    } else {
        let field = fields.first().unwrap();
        values.into_iter().map(|value| field.value(value)).collect()
    }
}

trait JsonFilterExt {
    fn json_contains(
        self,
        field: &ScalarFieldRef,
        value: ConditionValue,
        target_type: JsonTargetType,
        reverse: bool,
        alias: Option<Alias>,
        ctx: &Context<'_>,
    ) -> Expression<'static>;

    fn json_starts_with(
        self,
        field: &ScalarFieldRef,
        value: ConditionValue,
        target_type: JsonTargetType,
        reverse: bool,
        alias: Option<Alias>,
        ctx: &Context<'_>,
    ) -> Expression<'static>;

    fn json_ends_with(
        self,
        field: &ScalarFieldRef,
        value: ConditionValue,
        target_type: JsonTargetType,
        reverse: bool,
        alias: Option<Alias>,
        ctx: &Context<'_>,
    ) -> Expression<'static>;
}

impl JsonFilterExt for (Expression<'static>, Expression<'static>) {
    fn json_contains(
        self,
        field: &ScalarFieldRef,
        value: ConditionValue,
        target_type: JsonTargetType,
        reverse: bool,
        alias: Option<Alias>,
        ctx: &Context<'_>,
    ) -> Expression<'static> {
        let (expr_json, expr_string) = self;

        match (value, target_type) {
            // string_contains (value)
            (ConditionValue::Value(value), JsonTargetType::String) => {
                let contains = expr_string.like(format!("%{value}%"));

                if reverse {
                    contains.or(expr_json.json_type_not_equals(JsonType::String)).into()
                } else {
                    contains.and(expr_json.json_type_equals(JsonType::String)).into()
                }
            }
            // array_contains (value)
            (ConditionValue::Value(value), JsonTargetType::Array) => {
                let contains = expr_json.clone().json_array_contains(convert_pv(field, value));

                if reverse {
                    contains.or(expr_json.json_type_not_equals(JsonType::Array)).into()
                } else {
                    contains.and(expr_json.json_type_equals(JsonType::Array)).into()
                }
            }
            // string_contains (ref)
            (ConditionValue::FieldRef(field_ref), JsonTargetType::String) => {
                let contains = expr_string.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                    Value::text("%").raw().into(),
                    field_ref.aliased_col(alias, ctx).into(),
                    Value::text("%").raw().into(),
                ]));

                if reverse {
                    contains.or(expr_json.json_type_not_equals(JsonType::String)).into()
                } else {
                    contains.and(expr_json.json_type_equals(JsonType::String)).into()
                }
            }
            // array_contains (ref)
            (ConditionValue::FieldRef(field_ref), JsonTargetType::Array) => {
                let contains = expr_json.clone().json_array_contains(field_ref.aliased_col(alias, ctx));

                if reverse {
                    contains.or(expr_json.json_type_not_equals(JsonType::Array)).into()
                } else {
                    contains.and(expr_json.json_type_equals(JsonType::Array)).into()
                }
            }
        }
    }

    fn json_starts_with(
        self,
        field: &ScalarFieldRef,
        value: ConditionValue,
        target_type: JsonTargetType,
        reverse: bool,
        alias: Option<Alias>,
        ctx: &Context<'_>,
    ) -> Expression<'static> {
        let (expr_json, expr_string) = self;
        match (value, target_type) {
            // string_starts_with (value)
            (ConditionValue::Value(value), JsonTargetType::String) => {
                let starts_with = expr_string.like(format!("{value}%"));

                if reverse {
                    starts_with.or(expr_json.json_type_not_equals(JsonType::String)).into()
                } else {
                    starts_with.and(expr_json.json_type_equals(JsonType::String)).into()
                }
            }
            // array_starts_with (value)
            (ConditionValue::Value(value), JsonTargetType::Array) => {
                let starts_with = expr_json.clone().json_array_begins_with(convert_pv(field, value));

                if reverse {
                    starts_with.or(expr_json.json_type_not_equals(JsonType::Array)).into()
                } else {
                    starts_with.and(expr_json.json_type_equals(JsonType::Array)).into()
                }
            }
            // string_starts_with (ref)
            (ConditionValue::FieldRef(field_ref), JsonTargetType::String) => {
                let starts_with = expr_string.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                    field_ref.aliased_col(alias, ctx).into(),
                    Value::text("%").raw().into(),
                ]));

                if reverse {
                    starts_with.or(expr_json.json_type_not_equals(JsonType::String)).into()
                } else {
                    starts_with.and(expr_json.json_type_equals(JsonType::String)).into()
                }
            }
            // array_starts_with (ref)
            (ConditionValue::FieldRef(field_ref), JsonTargetType::Array) => {
                let starts_with = expr_json
                    .clone()
                    .json_array_begins_with(field_ref.aliased_col(alias, ctx));

                if reverse {
                    starts_with.or(expr_json.json_type_not_equals(JsonType::Array)).into()
                } else {
                    starts_with.and(expr_json.json_type_equals(JsonType::Array)).into()
                }
            }
        }
    }

    fn json_ends_with(
        self,
        field: &ScalarFieldRef,
        value: ConditionValue,
        target_type: JsonTargetType,
        reverse: bool,
        alias: Option<Alias>,
        ctx: &Context<'_>,
    ) -> Expression<'static> {
        let (expr_json, expr_string) = self;

        match (value, target_type) {
            // string_ends_with (value)
            (ConditionValue::Value(value), JsonTargetType::String) => {
                let ends_with = expr_string.like(format!("%{value}"));

                if reverse {
                    ends_with.or(expr_json.json_type_not_equals(JsonType::String)).into()
                } else {
                    ends_with.and(expr_json.json_type_equals(JsonType::String)).into()
                }
            }
            // array_ends_with (value)
            (ConditionValue::Value(value), JsonTargetType::Array) => {
                let ends_with = expr_json.clone().json_array_ends_into(convert_pv(field, value));

                if reverse {
                    ends_with.or(expr_json.json_type_not_equals(JsonType::Array)).into()
                } else {
                    ends_with.and(expr_json.json_type_equals(JsonType::Array)).into()
                }
            }
            // string_ends_with (ref)
            (ConditionValue::FieldRef(field_ref), JsonTargetType::String) => {
                let ends_with = expr_string.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                    Value::text("%").raw().into(),
                    field_ref.aliased_col(alias, ctx).into(),
                ]));

                if reverse {
                    ends_with.or(expr_json.json_type_not_equals(JsonType::String)).into()
                } else {
                    ends_with.and(expr_json.json_type_equals(JsonType::String)).into()
                }
            }
            // array_ends_with (ref)
            (ConditionValue::FieldRef(field_ref), JsonTargetType::Array) => {
                let ends_with = expr_json
                    .clone()
                    .json_array_ends_into(field_ref.aliased_col(alias, ctx));

                if reverse {
                    ends_with.or(expr_json.json_type_not_equals(JsonType::Array)).into()
                } else {
                    ends_with.and(expr_json.json_type_equals(JsonType::Array)).into()
                }
            }
        }
    }
}
