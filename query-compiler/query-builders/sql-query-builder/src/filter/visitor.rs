use super::alias::*;
use crate::join_utils::{AliasedJoin, compute_one2m_join};
use crate::value::Placeholder;
use crate::{Context, model_extensions::*};

use prisma_value::Placeholder as PrismaValuePlaceholder;
use psl::datamodel_connector::ConnectorCapability;
use psl::reachable_only_with_capability;
use quaint::ast::concat;
use quaint::ast::*;
use query_structure::{filter::*, prelude::*};
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
pub struct FilterVisitor {
    /// The parent alias, used when rendering nested filters so that a child filter can refer to its join.
    parent_alias: Option<Alias>,
    /// Whether filters can return top-level joins.
    with_top_level_joins: bool,
    /// Whether this visitor traverses nested filters.
    is_nested: bool,
    /// Whether the visitor is in a NOT clause.
    reverse: bool,
}

impl FilterVisitor {
    pub fn with_top_level_joins() -> Self {
        Self {
            with_top_level_joins: true,
            ..Default::default()
        }
    }

    pub fn without_top_level_joins() -> Self {
        Self {
            with_top_level_joins: false,
            ..Default::default()
        }
    }

    /// Returns the parent alias, if there's one set, so that nested filters can refer to the parent join/table.
    fn parent_alias(&self) -> Option<Alias> {
        self.parent_alias
    }

    #[cfg(feature = "relation_joins")]
    pub fn set_parent_alias_opt(mut self, alias: Option<Alias>) -> Self {
        self.parent_alias = alias;
        self
    }

    /// A top-level join can be rendered if we're explicitly allowing it or if we're in a nested visitor.
    fn can_render_join(&self) -> bool {
        self.with_top_level_joins || self.is_nested
    }

    /// Returns whether the visitor is in a NOT clause.
    fn reverse(&self) -> bool {
        self.reverse
    }

    fn invert_reverse<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        self.reverse = !self.reverse;
        let res = f(self);
        self.reverse = !self.reverse;
        res
    }

    fn create_nested_visitor(&self, parent_alias: Alias) -> Self {
        let mut nested_visitor = self.clone();
        nested_visitor.is_nested = true;
        nested_visitor.parent_alias = Some(parent_alias);

        nested_visitor
    }

    fn visit_nested_filter<T>(&mut self, parent_alias: Alias, f: impl FnOnce(&mut Self) -> T) -> T {
        let mut nested_visitor = self.create_nested_visitor(parent_alias);
        f(&mut nested_visitor)
    }

    fn visit_relation_filter_select(&mut self, filter: RelationFilter, ctx: &Context<'_>) -> Select<'static> {
        let is_many_to_many = filter.field.relation().is_many_to_many();
        // HACK: This is temporary. A fix should be done in Quaint instead of branching out here.
        // See https://www.notion.so/prismaio/Spec-Faulty-Tuple-Join-on-SQL-Server-55b8232fb44f4a6cb4d3f36428f17bac
        // for more info
        let support_row_in = filter
            .field
            .dm
            .schema
            .connector
            .capabilities()
            .contains(ConnectorCapability::RowIn);
        let has_compound_fields = filter.field.linking_fields().into_inner().len() > 1;

        // If the relation is an M2M relation we don't have a choice but to join
        // If the connector does not support (a, b) IN (SELECT c, d) and there are several linking fields, then we must use a join.
        // Hint: SQL Server does not support `ROW() IN ()`.
        if is_many_to_many || (!support_row_in && has_compound_fields) {
            self.visit_relation_filter_select_no_row(filter, ctx)
        } else {
            self.visit_relation_filter_select_with_row(filter, ctx)
        }
    }

    /// Traverses a relation filter using this rough SQL structure:
    ///
    /// ```sql
    /// EXISTS (
    ///   SELECT id FROM parent
    ///   INNER JOIN child ON (child.parent_id = parent.id)
    ///   WHERE <filter> AND outer.id = parent.id
    /// )
    /// ```
    /// We need this in two cases:
    /// - For M2M relations, as we need to traverse the join table so the join is not superfluous
    /// - SQL Server because it does not support (a, b) IN (subselect)
    fn visit_relation_filter_select_no_row(&mut self, filter: RelationFilter, ctx: &Context<'_>) -> Select<'static> {
        let table_alias = ctx.next_table_alias();
        let condition = filter.condition;
        let table = filter.field.as_table(ctx);
        let ids = ModelProjection::from(filter.field.model().primary_identifier());

        let selected_identifier: Vec<Column> = filter
            .field
            .identifier_columns(ctx)
            .map(|col| col.aliased_col(Some(table_alias), ctx))
            .collect();

        let join_columns: Vec<Column> = filter
            .field
            .join_columns(ctx)
            .map(|c| c.aliased_col(Some(table_alias), ctx))
            .collect();

        let related_table = filter.field.related_model().as_table(ctx);
        let related_join_columns: Vec<_> = ModelProjection::from(filter.field.related_field().linking_fields())
            .as_columns(ctx)
            .map(|col| col.aliased_col(Some(table_alias.to_join_alias()), ctx))
            .collect();

        let (nested_conditions, nested_joins) = self
            .visit_nested_filter(table_alias.to_join_alias(), |nested_visitor| {
                nested_visitor.visit_filter(*filter.nested_filter, ctx)
            });

        let parent_columns: Vec<_> = ids
            .as_columns(ctx)
            .map(|col| col.aliased_col(self.parent_alias(), ctx))
            .collect();

        let nested_conditions = nested_conditions
            .invert_if(condition.invert_of_subselect())
            .and(Row::from(parent_columns).equals(Row::from(selected_identifier.clone())));

        let nested_conditons = selected_identifier
            .clone()
            .into_iter()
            .fold(nested_conditions, |acc, column| acc.and(column.is_not_null()));

        let join = related_table
            .alias(table_alias.to_join_alias().to_string())
            .on(Row::from(related_join_columns).equals(Row::from(join_columns)));

        let select = Select::from_table(table.alias(table_alias.to_string()))
            .columns(selected_identifier)
            .inner_join(join)
            .so_that(nested_conditons);

        if let Some(nested_joins) = nested_joins {
            nested_joins.into_iter().fold(select, |acc, join| acc.join(join.data))
        } else {
            select
        }
    }

    /// Traverses a relation filter using this rough SQL structure:
    ///
    /// ```sql
    /// EXISTS (
    ///   SELECT id1, id2 FROM child
    ///   WHERE <filter> AND outer.id1 = child.id1 AND outer.id2 = child.id2
    /// )
    /// ```
    fn visit_relation_filter_select_with_row(&mut self, filter: RelationFilter, ctx: &Context<'_>) -> Select<'static> {
        let alias = ctx.next_table_alias();
        let condition = filter.condition;
        let linking_fields = ModelProjection::from(filter.field.linking_fields());

        let related_table = filter.field.related_model().as_table(ctx);
        // Select linking fields to match the linking fields of the parent record
        let related_columns: Vec<_> = filter
            .field
            .related_field()
            .join_columns(ctx)
            .map(|col| col.aliased_col(Some(alias), ctx))
            .collect();

        let (nested_conditions, nested_joins) =
            self.visit_nested_filter(alias, |this| this.visit_filter(*filter.nested_filter, ctx));
        let parent_columns: Vec<_> = linking_fields
            .as_columns(ctx)
            .map(|col| col.aliased_col(self.parent_alias(), ctx))
            .collect();

        let nested_conditions = nested_conditions
            .invert_if(condition.invert_of_subselect())
            .and(Row::from(parent_columns).equals(Row::from(related_columns.clone())));

        let conditions = related_columns
            .clone()
            .into_iter()
            .fold(nested_conditions, |acc, column| acc.and(column.is_not_null()));

        let select = Select::from_table(related_table.alias(alias.to_string()))
            .columns(related_columns)
            .so_that(conditions);

        if let Some(nested_joins) = nested_joins {
            nested_joins.into_iter().fold(select, |acc, join| acc.join(join.data))
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
                0 => (ConditionTree::NoCondition, None),
                1 => self.visit_filter(filters.pop().unwrap(), ctx),
                _ => {
                    let mut exprs = Vec::with_capacity(filters.len());
                    let mut top_level_joins = vec![];

                    for filter in filters {
                        let (conditions, nested_joins) = self.visit_filter(filter, ctx);

                        exprs.push(Expression::from(conditions));

                        if let Some(nested_joins) = nested_joins {
                            top_level_joins.extend(nested_joins);
                        }
                    }

                    (ConditionTree::And(exprs), Some(top_level_joins))
                }
            },
            Filter::Or(mut filters) => match filters.len() {
                0 => (ConditionTree::NegativeCondition, None),
                1 => self.visit_filter(filters.pop().unwrap(), ctx),
                _ => {
                    let mut exprs = Vec::with_capacity(filters.len());
                    let mut top_level_joins = vec![];

                    for filter in filters {
                        let (conditions, nested_joins) = self.visit_filter(filter, ctx);

                        exprs.push(Expression::from(conditions));

                        if let Some(nested_joins) = nested_joins {
                            top_level_joins.extend(nested_joins);
                        }
                    }

                    (ConditionTree::Or(exprs), Some(top_level_joins))
                }
            },
            Filter::Not(mut filters) => match filters.len() {
                0 => (ConditionTree::NoCondition, None),
                1 => {
                    let (cond, joins) = self.invert_reverse(|this| this.visit_filter(filters.pop().unwrap(), ctx));

                    (cond.not(), joins)
                }
                _ => {
                    let mut exprs = Vec::with_capacity(filters.len());
                    let mut top_level_joins = vec![];

                    for filter in filters {
                        let (conditions, nested_joins) = self.invert_reverse(|this| this.visit_filter(filter, ctx));
                        let inverted_conditions = conditions.not();

                        exprs.push(Expression::from(inverted_conditions));

                        if let Some(nested_joins) = nested_joins {
                            top_level_joins.extend(nested_joins);
                        }
                    }

                    (ConditionTree::And(exprs), Some(top_level_joins))
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
                reachable_only_with_capability!(ConnectorCapability::NativeFullTextSearch);
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
        let parent_alias = self.parent_alias().map(|a| a.to_string());

        match &filter.condition {
            // { to_one: { isNot: { ... } } }
            RelationCondition::NoRelatedRecord if self.can_render_join() && !filter.field.is_list() => {
                let alias = ctx.next_join_alias();

                let linking_fields_null: Vec<_> =
                    ModelProjection::from(filter.field.related_model().primary_identifier())
                        .as_columns(ctx)
                        .map(|c| c.aliased_col(Some(alias), ctx))
                        .map(|c| c.is_null())
                        .map(Expression::from)
                        .collect();
                let null_filter = ConditionTree::And(linking_fields_null);

                let join = compute_one2m_join(&filter.field, alias.to_string().as_str(), parent_alias.as_deref(), ctx);

                let mut output_joins = vec![join];

                let (conditions, nested_joins) = self.visit_nested_filter(alias, |nested_visitor| {
                    nested_visitor
                        .invert_reverse(|nested_visitor| nested_visitor.visit_filter(*filter.nested_filter, ctx))
                });

                if let Some(nested_joins) = nested_joins {
                    output_joins.extend(nested_joins);
                }

                (conditions.not().or(null_filter), Some(output_joins))
            }
            // { to_one: { is: { ... } } }
            RelationCondition::ToOneRelatedRecord if self.can_render_join() && !filter.field.is_list() => {
                let alias = ctx.next_join_alias();

                let linking_fields_not_null: Vec<_> =
                    ModelProjection::from(filter.field.related_model().primary_identifier())
                        .as_columns(ctx)
                        .map(|c| c.aliased_col(Some(alias), ctx))
                        .map(|c| c.is_not_null())
                        .map(Expression::from)
                        .collect();
                let not_null_filter = ConditionTree::And(linking_fields_not_null);

                let join = compute_one2m_join(&filter.field, alias.to_string().as_str(), parent_alias.as_deref(), ctx);
                let mut output_joins = vec![join];

                let (conditions, nested_joins) = self.visit_nested_filter(alias, |nested_visitor| {
                    nested_visitor.visit_filter(*filter.nested_filter, ctx)
                });

                if let Some(nested_joins) = nested_joins {
                    output_joins.extend(nested_joins);
                };

                (conditions.and(not_null_filter), Some(output_joins))
            }

            _ => {
                let condition = filter.condition;
                let sub_select = self.visit_relation_filter_select(filter, ctx);

                let comparison = match condition {
                    RelationCondition::AtLeastOneRelatedRecord | RelationCondition::ToOneRelatedRecord => {
                        Compare::Exists(Box::new(sub_select.into()))
                    }
                    RelationCondition::EveryRelatedRecord | RelationCondition::NoRelatedRecord => {
                        Compare::NotExists(Box::new(sub_select.into()))
                    }
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
        let parent_alias_string = parent_alias.as_ref().map(|a| a.to_string());

        // If the relation is inlined, we simply check whether the linking fields are null.
        //
        // ```sql
        //  SELECT "Parent"."id" FROM "Parent"
        //    WHERE "Parent"."childId" IS NULL;
        // ```
        if filter.field.is_inlined_on_enclosing_model() {
            let conditions: Vec<_> = ModelProjection::from(filter.field.linking_fields())
                .as_columns(ctx)
                .map(|c| c.opt_table(parent_alias_string.clone()))
                .map(|c| c.is_null())
                .map(Expression::from)
                .collect();

            return (ConditionTree::And(conditions), None);
        }

        // If the relation is not inlined and we can use joins, then we join the relation and check whether the related linking fields are null.
        //
        // ```sql
        //  SELECT "Parent"."id" FROM "Parent"
        //    LEFT JOIN "Child" AS "j1" ON ("j1"."parentId" = "Parent"."id")
        //  WHERE "j1"."parentId" IS NULL OFFSET;
        // ```
        if self.can_render_join() {
            let alias = ctx.next_join_alias();

            let conditions: Vec<_> = ModelProjection::from(filter.field.related_field().linking_fields())
                .as_columns(ctx)
                .map(|c| c.aliased_col(Some(alias), ctx))
                .map(|c| c.is_null())
                .map(Expression::from)
                .collect();

            let join = compute_one2m_join(
                &filter.field,
                alias.to_string().as_str(),
                parent_alias_string.as_deref(),
                ctx,
            );

            return (ConditionTree::And(conditions), Some(vec![join]));
        }

        // Otherwise, we use a NOT IN clause and a subselect to find the related records that are nulls.
        //
        // ```sql
        //  SELECT "Parent"."id" FROM "Parent"
        //    WHERE ("Parent".id) NOT IN (
        //      SELECT "Child"."parentId" FROM "Child" WHERE "Child"."parentId" IS NOT NULL
        //    )
        // ```
        let relation = filter.field.relation();
        let table = relation.as_table(ctx);
        let relation_table = match parent_alias {
            Some(ref alias) => table.alias(alias.to_table_alias().to_string()),
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
        reachable_only_with_capability!(ConnectorCapability::ScalarLists);

        let comparable: Expression = filter.field.aliased_col(self.parent_alias(), ctx).into();
        let cond = filter.condition;
        let field = &filter.field;
        let alias = self.parent_alias();

        let condition = match cond {
            ScalarListCondition::Contains(ConditionValue::Value(val)) => {
                comparable.compare_raw("@>", convert_list_pv(field, vec![val], ctx))
            }
            ScalarListCondition::Contains(ConditionValue::FieldRef(field_ref)) => {
                let field_ref_expr: Expression = field_ref.aliased_col(alias, ctx).into();

                // This code path is only reachable for connectors with `ScalarLists` capability
                field_ref_expr.equals(comparable.any())
            }
            ScalarListCondition::ContainsEvery(ConditionListValue::List(vals)) => {
                comparable.compare_raw("@>", convert_list_pv(field, vals, ctx))
            }
            ScalarListCondition::ContainsEvery(ConditionListValue::FieldRef(field_ref)) => {
                comparable.compare_raw("@>", field_ref.aliased_col(alias, ctx))
            }
            ScalarListCondition::ContainsSome(ConditionListValue::List(vals)) => {
                comparable.compare_raw("&&", convert_list_pv(field, vals, ctx))
            }
            ScalarListCondition::ContainsSome(ConditionListValue::FieldRef(field_ref)) => {
                comparable.compare_raw("&&", field_ref.aliased_col(alias, ctx))
            }
            ScalarListCondition::ContainsEvery(ConditionListValue::Placeholder(placeholder)) => {
                let param: Expression = field.value(placeholder.into(), ctx).into();
                comparable.compare_raw("@>", param)
            }
            ScalarListCondition::ContainsSome(ConditionListValue::Placeholder(placeholder)) => {
                let param: Expression = field.value(placeholder.into(), ctx).into();
                comparable.compare_raw("&&", param)
            }
            ScalarListCondition::IsEmpty(true) => comparable.compare_raw("=", ValueType::Array(Some(vec![])).raw()),
            ScalarListCondition::IsEmpty(false) => comparable.compare_raw("<>", ValueType::Array(Some(vec![])).raw()),
        };

        ConditionTree::single(condition)
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
    let sf = filter.into_scalar().unwrap();

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
        ScalarCondition::JsonCompare(json_compare) => {
            reachable_only_with_capability!(ConnectorCapability::JsonFiltering);
            convert_json_filter(
                comparable,
                json_compare,
                reverse,
                fields.first().unwrap(),
                mode,
                alias,
                ctx,
            )
        }
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
            (expr_json, expr_string).json_contains(field, value, target_type.unwrap(), query_mode, reverse, alias, ctx)
        }
        ScalarCondition::StartsWith(value) => (expr_json, expr_string).json_starts_with(
            field,
            value,
            target_type.unwrap(),
            query_mode,
            reverse,
            alias,
            ctx,
        ),
        ScalarCondition::EndsWith(value) => {
            (expr_json, expr_string).json_ends_with(field, value, target_type.unwrap(), query_mode, reverse, alias, ctx)
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
                std::slice::from_ref(field),
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

pub(crate) fn default_scalar_filter(
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
            ConditionValue::Value(value) => comparable.like(like_contains_pattern(value)),
            ConditionValue::FieldRef(field_ref) => comparable.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                Value::text("%").raw().into(),
                field_ref.aliased_col(alias, ctx).into(),
                Value::text("%").raw().into(),
            ])),
        },
        ScalarCondition::NotContains(value) => match value {
            ConditionValue::Value(value) => comparable.not_like(like_contains_pattern(value)),
            ConditionValue::FieldRef(field_ref) => {
                comparable.not_like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                    Value::text("%").raw().into(),
                    field_ref.aliased_col(alias, ctx).into(),
                    Value::text("%").raw().into(),
                ]))
            }
        },
        ScalarCondition::StartsWith(value) => match value {
            ConditionValue::Value(value) => comparable.like(like_starts_with_pattern(value)),
            ConditionValue::FieldRef(field_ref) => comparable.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                field_ref.aliased_col(alias, ctx).into(),
                Value::text("%").raw().into(),
            ])),
        },
        ScalarCondition::NotStartsWith(value) => match value {
            ConditionValue::Value(value) => comparable.not_like(like_starts_with_pattern(value)),
            ConditionValue::FieldRef(field_ref) => {
                comparable.not_like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                    field_ref.aliased_col(alias, ctx).into(),
                    Value::text("%").raw().into(),
                ]))
            }
        },
        ScalarCondition::EndsWith(value) => match value {
            ConditionValue::Value(value) => comparable.like(like_ends_with_pattern(value)),
            ConditionValue::FieldRef(field_ref) => comparable.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                Value::text("%").raw().into(),
                field_ref.aliased_col(alias, ctx).into(),
            ])),
        },
        ScalarCondition::NotEndsWith(value) => match value {
            ConditionValue::Value(value) => comparable.not_like(like_ends_with_pattern(value)),
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
                    let list_value = convert_pvs(fields, pv.into_list().unwrap(), ctx);
                    sql_values.push(list_value);
                }

                comparable.in_selection(sql_values)
            }
            _ => comparable.in_selection(convert_pvs(fields, values, ctx)),
        },
        ScalarCondition::In(ConditionListValue::FieldRef(field_ref)) => {
            // This code path is only reachable for connectors with `ScalarLists` capability
            comparable.equals(Expression::from(field_ref.aliased_col(alias, ctx)).any())
        }
        ScalarCondition::In(ConditionListValue::Placeholder(placeholder)) => {
            let sql_value = convert_first_value(fields, PrismaValue::from(placeholder), alias, ctx);
            comparable.in_selection(sql_value.into_parameterized_row())
        }
        ScalarCondition::NotIn(ConditionListValue::List(values)) => match values.split_first() {
            Some((PrismaValue::List(_), _)) => {
                let mut sql_values = Values::with_capacity(values.len());

                for pv in values {
                    let list_value = convert_pvs(fields, pv.into_list().unwrap(), ctx);
                    sql_values.push(list_value);
                }

                comparable.not_in_selection(sql_values)
            }
            _ => comparable.not_in_selection(convert_pvs(fields, values, ctx)),
        },
        ScalarCondition::NotIn(ConditionListValue::FieldRef(field_ref)) => {
            // This code path is only reachable for connectors with `ScalarLists` capability
            comparable.not_equals(Expression::from(field_ref.aliased_col(alias, ctx)).all())
        }
        ScalarCondition::NotIn(ConditionListValue::Placeholder(placeholder)) => {
            let sql_value = convert_first_value(fields, PrismaValue::from(placeholder), alias, ctx);
            comparable.not_in_selection(sql_value.into_parameterized_row())
        }
        ScalarCondition::Search(value, _) => {
            reachable_only_with_capability!(ConnectorCapability::NativeFullTextSearch);
            let query = prisma_value_to_search_expression(value.into_value().unwrap());

            comparable.matches(query)
        }
        ScalarCondition::NotSearch(value, _) => {
            reachable_only_with_capability!(ConnectorCapability::NativeFullTextSearch);
            let query = prisma_value_to_search_expression(value.into_value().unwrap());

            comparable.not_matches(query)
        }
        ScalarCondition::JsonCompare(_) => unreachable!(),
        ScalarCondition::IsSet(_) => unreachable!(),
    };

    ConditionTree::single(condition)
}

fn prisma_value_to_search_expression(pv: PrismaValue) -> Expression<'static> {
    match pv {
        PrismaValue::String(s) => Value::text(s).into(),
        PrismaValue::Placeholder(PrismaValuePlaceholder { name, .. }) => {
            Value::opaque(Placeholder::new(name), OpaqueType::Text).into()
        }
        _ => panic!("Search field should only contain String or Placeholder values"),
    }
}

/// Converts a PrismaValue to an Expression for use in LIKE patterns.
fn prisma_value_to_like_expression(pv: PrismaValue) -> Expression<'static> {
    match pv {
        PrismaValue::String(s) => Value::text(s).into(),
        PrismaValue::Placeholder(PrismaValuePlaceholder { name, .. }) => {
            Value::opaque(Placeholder::new(name), OpaqueType::Text).into()
        }
        _ => panic!("LIKE filter value should be String or Placeholder"),
    }
}

/// Creates a LIKE pattern expression for "contains" filter (%value%).
/// For concrete strings, returns a simple formatted string.
/// For placeholders, returns CONCAT('%', $placeholder, '%').
fn like_contains_pattern(value: PrismaValue) -> Expression<'static> {
    match value {
        PrismaValue::String(s) => format!("%{s}%").into(),
        PrismaValue::Placeholder(_) => concat(vec![
            Value::text("%").raw().into(),
            prisma_value_to_like_expression(value),
            Value::text("%").raw().into(),
        ])
        .into(),
        _ => panic!("LIKE filter value should be String or Placeholder"),
    }
}

/// Creates a LIKE pattern expression for "starts with" filter (value%).
/// For concrete strings, returns a simple formatted string.
/// For placeholders, returns CONCAT($placeholder, '%').
fn like_starts_with_pattern(value: PrismaValue) -> Expression<'static> {
    match value {
        PrismaValue::String(s) => format!("{s}%").into(),
        PrismaValue::Placeholder(_) => concat(vec![
            prisma_value_to_like_expression(value),
            Value::text("%").raw().into(),
        ])
        .into(),
        _ => panic!("LIKE filter value should be String or Placeholder"),
    }
}

/// Creates a LIKE pattern expression for "ends with" filter (%value).
/// For concrete strings, returns a simple formatted string.
/// For placeholders, returns CONCAT('%', $placeholder).
fn like_ends_with_pattern(value: PrismaValue) -> Expression<'static> {
    match value {
        PrismaValue::String(s) => format!("%{s}").into(),
        PrismaValue::Placeholder(_) => concat(vec![
            Value::text("%").raw().into(),
            prisma_value_to_like_expression(value),
        ])
        .into(),
        _ => panic!("LIKE filter value should be String or Placeholder"),
    }
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
            ConditionValue::Value(value) => comparable.compare_raw("ILIKE", prisma_value_to_like_expression(value)),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw("ILIKE", field_ref.aliased_col(alias, ctx)),
        },
        ScalarCondition::NotEquals(ConditionValue::Value(PrismaValue::Null)) => comparable.is_not_null(),
        ScalarCondition::NotEquals(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("NOT ILIKE", prisma_value_to_like_expression(value)),
            ConditionValue::FieldRef(field_ref) => {
                comparable.compare_raw("NOT ILIKE", field_ref.aliased_col(alias, ctx))
            }
        },
        ScalarCondition::Contains(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("ILIKE", like_contains_pattern(value)),
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
            ConditionValue::Value(value) => comparable.compare_raw("NOT ILIKE", like_contains_pattern(value)),
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
            ConditionValue::Value(value) => comparable.compare_raw("ILIKE", like_starts_with_pattern(value)),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw(
                "ILIKE",
                concat::<'_, Expression<'_>>(vec![field_ref.aliased_col(alias, ctx).into(), Value::text("%").into()]),
            ),
        },
        ScalarCondition::NotStartsWith(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("NOT ILIKE", like_starts_with_pattern(value)),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw(
                "NOT ILIKE",
                concat::<'_, Expression<'_>>(vec![field_ref.aliased_col(alias, ctx).into(), Value::text("%").into()]),
            ),
        },
        ScalarCondition::EndsWith(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("ILIKE", like_ends_with_pattern(value)),
            ConditionValue::FieldRef(field_ref) => comparable.compare_raw(
                "ILIKE",
                concat::<'_, Expression<'_>>(vec![Value::text("%").into(), field_ref.aliased_col(alias, ctx).into()]),
            ),
        },
        ScalarCondition::NotEndsWith(value) => match value {
            ConditionValue::Value(value) => comparable.compare_raw("NOT ILIKE", like_ends_with_pattern(value)),
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
                    let list_value = convert_pvs(fields, pv.into_list().unwrap(), ctx);
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
        ScalarCondition::In(ConditionListValue::Placeholder(placeholder)) => {
            let comparable = Expression::from(lower(comparable));
            let sql_value = convert_first_value(fields, PrismaValue::from(placeholder), alias, ctx);
            comparable.in_selection(lower(sql_value.into_parameterized_row()))
        }
        ScalarCondition::NotIn(ConditionListValue::List(values)) => match values.split_first() {
            Some((PrismaValue::List(_), _)) => {
                let mut sql_values = Values::with_capacity(values.len());

                for pv in values {
                    let list_value = convert_pvs(fields, pv.into_list().unwrap(), ctx);
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
        ScalarCondition::NotIn(ConditionListValue::Placeholder(placeholder)) => {
            let comparable = Expression::from(lower(comparable));
            let sql_value = convert_first_value(fields, PrismaValue::from(placeholder), alias, ctx);
            comparable.not_in_selection(lower(sql_value.into_parameterized_row()))
        }
        ScalarCondition::Search(value, _) => {
            reachable_only_with_capability!(ConnectorCapability::NativeFullTextSearch);
            let query: String = value
                .into_value()
                .unwrap()
                .try_into()
                .unwrap_or_else(|err: ConversionFailure| panic!("{}", err));

            comparable.matches(query)
        }
        ScalarCondition::NotSearch(value, _) => {
            reachable_only_with_capability!(ConnectorCapability::NativeFullTextSearch);
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
    if cond { lower(expr).into() } else { expr }
}

fn convert_value<'a>(
    field: &ScalarFieldRef,
    value: impl Into<ConditionValue>,
    alias: Option<Alias>,
    ctx: &Context<'_>,
) -> Expression<'a> {
    match value.into() {
        ConditionValue::Value(pv) => convert_pv(field, pv, ctx),
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
        ConditionValue::Value(pv) => convert_pv(fields.first().unwrap(), pv, ctx),
        ConditionValue::FieldRef(field_ref) => field_ref.aliased_col(alias, ctx).into(),
    }
}

fn convert_pv<'a>(field: &ScalarFieldRef, pv: PrismaValue, ctx: &Context<'_>) -> Expression<'a> {
    field.value(pv, ctx).into()
}

fn convert_list_pv<'a>(field: &ScalarFieldRef, values: Vec<PrismaValue>, ctx: &Context<'_>) -> Expression<'a> {
    Expression::from(Value::array(values.into_iter().map(|val| field.value(val, ctx))))
}

fn convert_pvs<'a>(fields: &[ScalarFieldRef], values: Vec<PrismaValue>, ctx: &Context<'_>) -> Vec<Value<'a>> {
    if fields.len() == values.len() {
        fields
            .iter()
            .zip(values)
            .map(|(field, value)| field.value(value, ctx))
            .collect()
    } else {
        let field = fields.first().unwrap();
        values.into_iter().map(|value| field.value(value, ctx)).collect()
    }
}

trait JsonFilterExt {
    #[allow(clippy::too_many_arguments)]
    fn json_contains(
        self,
        field: &ScalarFieldRef,
        value: ConditionValue,
        target_type: JsonTargetType,
        query_mode: QueryMode,
        reverse: bool,
        alias: Option<Alias>,
        ctx: &Context<'_>,
    ) -> Expression<'static>;

    #[allow(clippy::too_many_arguments)]
    fn json_starts_with(
        self,
        field: &ScalarFieldRef,
        value: ConditionValue,
        target_type: JsonTargetType,
        query_mode: QueryMode,
        reverse: bool,
        alias: Option<Alias>,
        ctx: &Context<'_>,
    ) -> Expression<'static>;

    #[allow(clippy::too_many_arguments)]
    fn json_ends_with(
        self,
        field: &ScalarFieldRef,
        value: ConditionValue,
        target_type: JsonTargetType,
        query_mode: QueryMode,
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
        query_mode: QueryMode,
        reverse: bool,
        alias: Option<Alias>,
        ctx: &Context<'_>,
    ) -> Expression<'static> {
        let (expr_json, expr_string) = self;

        match (value, target_type) {
            // string_contains (value)
            (ConditionValue::Value(value), JsonTargetType::String) => {
                let contains = match query_mode {
                    QueryMode::Default => expr_string.like(like_contains_pattern(value)),
                    QueryMode::Insensitive => {
                        Expression::from(lower(expr_string)).like(lower(like_contains_pattern(value)))
                    }
                };

                if reverse {
                    contains.or(expr_json.json_type_not_equals(JsonType::String)).into()
                } else {
                    contains.and(expr_json.json_type_equals(JsonType::String)).into()
                }
            }
            // array_contains (value)
            (ConditionValue::Value(value), JsonTargetType::Array) => {
                reachable_only_with_capability!(ConnectorCapability::JsonArrayContains);

                let contains = expr_json.clone().json_array_contains(convert_pv(field, value, ctx));

                if reverse {
                    contains.or(expr_json.json_type_not_equals(JsonType::Array)).into()
                } else {
                    contains.and(expr_json.json_type_equals(JsonType::Array)).into()
                }
            }
            // string_contains (ref)
            (ConditionValue::FieldRef(field_ref), JsonTargetType::String) => {
                let contains =
                    match query_mode {
                        QueryMode::Default => expr_string.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                            Value::text("%").raw().into(),
                            field_ref.aliased_col(alias, ctx).into(),
                            Value::text("%").raw().into(),
                        ])),
                        QueryMode::Insensitive => Expression::from(lower(expr_string)).like(lower(
                            quaint::ast::concat::<'_, Expression<'_>>(vec![
                                Value::text("%").raw().into(),
                                field_ref.aliased_col(alias, ctx).into(),
                                Value::text("%").raw().into(),
                            ]),
                        )),
                    };

                if reverse {
                    contains.or(expr_json.json_type_not_equals(JsonType::String)).into()
                } else {
                    contains.and(expr_json.json_type_equals(JsonType::String)).into()
                }
            }
            // array_contains (ref)
            (ConditionValue::FieldRef(field_ref), JsonTargetType::Array) => {
                reachable_only_with_capability!(ConnectorCapability::JsonArrayContains);

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
        query_mode: QueryMode,
        reverse: bool,
        alias: Option<Alias>,
        ctx: &Context<'_>,
    ) -> Expression<'static> {
        let (expr_json, expr_string) = self;
        match (value, target_type) {
            // string_starts_with (value)
            (ConditionValue::Value(value), JsonTargetType::String) => {
                let starts_with = match query_mode {
                    QueryMode::Default => expr_string.like(like_starts_with_pattern(value)),
                    QueryMode::Insensitive => {
                        Expression::from(lower(expr_string)).like(lower(like_starts_with_pattern(value)))
                    }
                };

                if reverse {
                    starts_with.or(expr_json.json_type_not_equals(JsonType::String)).into()
                } else {
                    starts_with.and(expr_json.json_type_equals(JsonType::String)).into()
                }
            }
            // array_starts_with (value)
            (ConditionValue::Value(value), JsonTargetType::Array) => {
                let starts_with = expr_json.clone().json_array_begins_with(convert_pv(field, value, ctx));

                if reverse {
                    starts_with.or(expr_json.json_type_not_equals(JsonType::Array)).into()
                } else {
                    starts_with.and(expr_json.json_type_equals(JsonType::Array)).into()
                }
            }
            // string_starts_with (ref)
            (ConditionValue::FieldRef(field_ref), JsonTargetType::String) => {
                let starts_with = match query_mode {
                    QueryMode::Default => expr_string.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                        field_ref.aliased_col(alias, ctx).into(),
                        Value::text("%").raw().into(),
                    ])),
                    QueryMode::Insensitive => {
                        Expression::from(lower(expr_string)).like(lower(quaint::ast::concat::<'_, Expression<'_>>(
                            vec![field_ref.aliased_col(alias, ctx).into(), Value::text("%").raw().into()],
                        )))
                    }
                };

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
        query_mode: QueryMode,
        reverse: bool,
        alias: Option<Alias>,
        ctx: &Context<'_>,
    ) -> Expression<'static> {
        let (expr_json, expr_string) = self;

        match (value, target_type) {
            // string_ends_with (value)
            (ConditionValue::Value(value), JsonTargetType::String) => {
                let ends_with = match query_mode {
                    QueryMode::Default => expr_string.like(like_ends_with_pattern(value)),
                    QueryMode::Insensitive => {
                        Expression::from(lower(expr_string)).like(lower(like_ends_with_pattern(value)))
                    }
                };

                if reverse {
                    ends_with.or(expr_json.json_type_not_equals(JsonType::String)).into()
                } else {
                    ends_with.and(expr_json.json_type_equals(JsonType::String)).into()
                }
            }
            // array_ends_with (value)
            (ConditionValue::Value(value), JsonTargetType::Array) => {
                let ends_with = expr_json.clone().json_array_ends_into(convert_pv(field, value, ctx));

                if reverse {
                    ends_with.or(expr_json.json_type_not_equals(JsonType::Array)).into()
                } else {
                    ends_with.and(expr_json.json_type_equals(JsonType::Array)).into()
                }
            }
            // string_ends_with (ref)
            (ConditionValue::FieldRef(field_ref), JsonTargetType::String) => {
                let ends_with = match query_mode {
                    QueryMode::Default => expr_string.like(quaint::ast::concat::<'_, Expression<'_>>(vec![
                        Value::text("%").raw().into(),
                        field_ref.aliased_col(alias, ctx).into(),
                    ])),
                    QueryMode::Insensitive => {
                        Expression::from(lower(expr_string)).like(lower(quaint::ast::concat::<'_, Expression<'_>>(
                            vec![Value::text("%").raw().into(), field_ref.aliased_col(alias, ctx).into()],
                        )))
                    }
                };

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
