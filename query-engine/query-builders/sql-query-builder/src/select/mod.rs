mod lateral;
mod subquery;

use itertools::{Either, Itertools};
use std::borrow::Cow;

use psl::{
    datamodel_connector::{ConnectorCapability, Flavour},
    has_capability,
};
use quaint::prelude::*;
use query_structure::*;

use crate::{
    context::Context,
    filter::alias::Alias,
    model_extensions::{AsColumn, AsColumns, AsTable, ColumnIterator, RelationFieldExt},
    ordering::OrderByBuilder,
    sql_trace::SqlTraceComment,
};

use self::{lateral::LateralJoinSelectBuilder, subquery::SubqueriesSelectBuilder};

pub(crate) const JSON_AGG_IDENT: &str = "__prisma_data__";

pub struct SelectBuilder;

impl SelectBuilder {
    pub fn build(args: QueryArguments, selected_fields: &FieldSelection, ctx: &Context<'_>) -> Select<'static> {
        if supports_lateral_join(&args) {
            LateralJoinSelectBuilder::default().build(args, selected_fields, ctx)
        } else {
            SubqueriesSelectBuilder.build(args, selected_fields, ctx)
        }
    }
}

pub(crate) trait JoinSelectBuilder {
    /// Build the select query for the given query arguments and selected fields.
    /// This is the entry point for building a select query. `build_default_select` can be used to get a default select query.
    fn build(&mut self, args: QueryArguments, selected_fields: &FieldSelection, ctx: &Context<'_>) -> Select<'static>;
    /// Adds to `select` the SQL statements to fetch a 1-1 relation.
    fn add_to_one_relation<'a>(
        &mut self,
        select: Select<'a>,
        rs: &RelationSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a>;
    /// Adds to `select` the SQL statements to fetch a 1-m relation.
    fn add_to_many_relation<'a, 'b>(
        &mut self,
        select: Select<'a>,
        rs: &RelationSelection,
        parent_virtuals: impl Iterator<Item = &'b VirtualSelection>,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a>;
    /// Adds to `select` the SQL statements to fetch a m-n relation.
    fn add_many_to_many_relation<'a, 'b>(
        &mut self,
        select: Select<'a>,
        rs: &RelationSelection,
        parent_virtuals: impl Iterator<Item = &'b VirtualSelection>,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a>;
    fn add_virtual_selection<'a>(
        &mut self,
        select: Select<'a>,
        vs: &VirtualSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a>;
    /// Build the top-level selection set
    fn build_selection<'a>(
        &mut self,
        select: Select<'a>,
        field: &SelectedField,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a>;
    fn build_json_obj_fn(
        &mut self,
        rs: &RelationSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Expression<'static>;
    fn build_virtual_expr(
        &mut self,
        vs: &VirtualSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Expression<'static>;
    /// Checks if a virtual selection has already been added to the query at an earlier stage
    /// as a part of a relation query for a matching relation field.
    fn was_virtual_processed_in_relation(&self, vs: &VirtualSelection) -> bool;

    fn with_selection<'a>(
        &mut self,
        select: Select<'a>,
        selected_fields: &FieldSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        let select = selected_fields.selections().fold(select, |acc, selection| {
            self.build_selection(acc, selection, parent_alias, ctx)
        });

        self.build_json_obj_virtual_selection(selected_fields.virtuals(), parent_alias, ctx)
            .into_iter()
            .fold(select, |acc, (alias, expr)| acc.value(expr.alias(alias)))
    }

    /// Builds the core select for a 1-1 relation.
    /// Note: it does not add the JSON object selection because there are additional steps to
    /// perform before that depending on the `JoinSelectBuilder` implementation.
    fn build_to_one_select(
        &mut self,
        rs: &RelationSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> (Select<'static>, Alias) {
        let rf = &rs.field;
        let child_table_alias = ctx.next_table_alias();
        let table = rs
            .field
            .related_field()
            .as_table(ctx)
            .alias(child_table_alias.to_string());

        let select = Select::from_table(table)
            .with_join_conditions(rf, parent_alias, child_table_alias, ctx)
            .with_filters(rs.args.filter.clone(), Some(child_table_alias), ctx)
            .limit(1);

        (select, child_table_alias)
    }

    /// Builds the core select for a 1-m relation.
    fn build_to_many_select(
        &mut self,
        rs: &RelationSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'static> {
        let inner_root_table_alias = ctx.next_table_alias();
        let root_alias = ctx.next_table_alias();
        let inner_alias = ctx.next_table_alias();
        let middle_alias = ctx.next_table_alias();

        let related_table = rs
            .related_model()
            .as_table(ctx)
            .alias(inner_root_table_alias.to_string());

        // SELECT * FROM "Table" as <table_alias> WHERE parent.id = child.parent_id
        let root = Select::from_table(related_table)
            .with_join_conditions(&rs.field, parent_alias, inner_root_table_alias, ctx)
            .comment("root select");

        // SELECT JSON_BUILD_OBJECT() FROM ( <root> )
        let inner = Select::from_table(Table::from(root).alias(root_alias.to_string()));
        let inner = self.with_relations(inner, rs.relations(), rs.virtuals(), root_alias, ctx);
        let inner = self.with_virtual_selections(inner, rs.virtuals(), root_alias, ctx);

        // Build the JSON object utilizing the information we collected in `with_relations` and
        // `with_virtual_selections`.
        let inner = inner.value(self.build_json_obj_fn(rs, root_alias, ctx).alias(JSON_AGG_IDENT));

        let linking_fields = rs.field.related_field().linking_fields();

        if rs.field.relation().is_many_to_many() {
            let selection: Vec<Column<'_>> = FieldSelection::union(vec![
                order_by_selection(rs),
                distinct_selection(rs),
                linking_fields,
                filtering_selection(rs),
            ])
            .into_projection()
            .as_columns(ctx)
            .map(|c| c.table(root_alias.to_string()))
            .collect();

            // SELECT <foreign_keys>, <orderby columns>
            inner.with_columns(selection.into())
        } else {
            // select ordering, distinct, filtering & join fields from child selections to order,
            // filter & join them on the outer query
            let inner_selection: Vec<Column<'_>> = FieldSelection::union(vec![
                order_by_selection(rs),
                distinct_selection(rs),
                filtering_selection(rs),
                relation_selection(rs),
            ])
            .into_projection()
            .as_columns(ctx)
            .map(|c| c.table(root_alias.to_string()))
            .collect();

            let inner = inner.with_columns(inner_selection.into()).comment("inner select");

            let override_empty_middle_take = match connector_flavour(&rs.args) {
                // On MySQL, using LIMIT makes the ordering of the JSON_AGG working. Beware, this is undocumented behavior.
                // Note: Ideally, this should live in the MySQL select builder, but it's currently the only implementation difference
                // between MySQL and Postgres, so we keep it here for now to avoid code duplication.
                Flavour::Mysql if !rs.args.order_by.is_empty() => Some(i64::MAX),
                _ => None,
            };

            let middle = Select::from_table(Table::from(inner).alias(inner_alias.to_string()))
                // SELECT <inner_alias>.<JSON_ADD_IDENT>
                .column(Column::from((inner_alias.to_string(), JSON_AGG_IDENT)))
                // DISTINCT ON
                .with_distinct(&rs.args, inner_alias)
                // ORDER BY ...
                .with_ordering(&rs.args, Some(inner_alias.to_string()), ctx)
                // WHERE ...
                .with_filters(rs.args.filter.clone(), Some(inner_alias), ctx)
                // LIMIT $1 OFFSET $2
                .with_pagination(&rs.args, override_empty_middle_take)
                .comment("middle select");

            // SELECT COALESCE(JSON_AGG(<inner_alias>), '[]') AS <inner_alias> FROM ( <middle> ) as <inner_alias_2>
            Select::from_table(Table::from(middle).alias(middle_alias.to_string()))
                .value(json_agg())
                .comment("outer select")
        }
    }

    fn with_relation<'a, 'b>(
        &mut self,
        select: Select<'a>,
        rs: &RelationSelection,
        parent_virtuals: impl Iterator<Item = &'b VirtualSelection>,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        match (rs.field.is_list(), rs.field.relation().is_many_to_many()) {
            (true, true) => self.add_many_to_many_relation(select, rs, parent_virtuals, parent_alias, ctx),
            (true, false) => self.add_to_many_relation(select, rs, parent_virtuals, parent_alias, ctx),
            (false, _) => self.add_to_one_relation(select, rs, parent_alias, ctx),
        }
    }

    fn with_relations<'a, 'b>(
        &mut self,
        input: Select<'a>,
        relation_selections: impl Iterator<Item = &'b RelationSelection>,
        virtual_selections: impl Iterator<Item = &'b VirtualSelection>,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        let virtual_selections = virtual_selections.collect::<Vec<_>>();

        relation_selections.fold(input, |acc, rs| {
            self.with_relation(acc, rs, virtual_selections.iter().copied(), parent_alias, ctx)
        })
    }

    fn build_default_select(&mut self, args: &QueryArguments, ctx: &Context<'_>) -> (Select<'static>, Alias) {
        let table_alias = ctx.next_table_alias();
        let table = args.model().as_table(ctx).alias(table_alias.to_string());

        // SELECT ... FROM Table "t1"
        let select = Select::from_table(table)
            .with_distinct(args, table_alias)
            .with_ordering(args, Some(table_alias.to_string()), ctx)
            .with_filters(args.filter.clone(), Some(table_alias), ctx)
            .with_pagination(args, None)
            .add_traceparent(ctx.traceparent);

        (select, table_alias)
    }

    fn with_virtual_selections<'a, 'b>(
        &mut self,
        select: Select<'a>,
        selections: impl Iterator<Item = &'b VirtualSelection>,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        selections.fold(select, |acc, vs| {
            if self.was_virtual_processed_in_relation(vs) {
                acc
            } else {
                self.add_virtual_selection(acc, vs, parent_alias, ctx)
            }
        })
    }

    fn build_virtual_select(
        &mut self,
        vs: &VirtualSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'static> {
        match vs {
            VirtualSelection::RelationCount(rf, filter) => {
                if rf.relation().is_many_to_many() {
                    self.build_relation_count_query_m2m(vs.db_alias(), rf, filter, parent_alias, ctx)
                } else {
                    self.build_relation_count_query(vs.db_alias(), rf, filter, parent_alias, ctx)
                }
            }
        }
    }

    fn build_json_obj_virtual_selection<'a>(
        &mut self,
        virtual_fields: impl Iterator<Item = &'a VirtualSelection>,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Vec<(Cow<'static, str>, Expression<'static>)> {
        let mut selected_objects = std::collections::BTreeMap::new();

        for vs in virtual_fields {
            let (object_name, field_name) = vs.serialized_name();
            let virtual_expr = self.build_virtual_expr(vs, parent_alias, ctx);

            selected_objects
                .entry(object_name)
                .or_insert(Vec::new())
                .push((field_name.to_owned().into(), virtual_expr));
        }

        selected_objects
            .into_iter()
            .map(|(name, fields)| (name.into(), json_build_object(fields).into()))
            .collect()
    }

    fn build_relation_count_query<'a>(
        &mut self,
        selection_name: impl Into<Cow<'static, str>>,
        rf: &RelationField,
        filter: &Option<Filter>,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        let related_table_alias = ctx.next_table_alias();

        let related_table = rf.related_model().as_table(ctx).alias(related_table_alias.to_string());

        Select::from_table(related_table)
            .value(count(asterisk()).alias(selection_name))
            .with_join_conditions(rf, parent_alias, related_table_alias, ctx)
            .with_filters(filter.clone(), Some(related_table_alias), ctx)
    }

    fn build_relation_count_query_m2m<'a>(
        &mut self,
        selection_name: impl Into<Cow<'static, str>>,
        rf: &RelationField,
        filter: &Option<Filter>,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        let related_table_alias = ctx.next_table_alias();
        let m2m_table_alias = ctx.next_table_alias();

        let related_table = rf.related_model().as_table(ctx).alias(related_table_alias.to_string());

        let m2m_join_conditions = {
            let left_columns = rf.join_columns(ctx);
            let right_columns = ModelProjection::from(rf.related_field().linking_fields()).as_columns(ctx);
            build_join_conditions(
                (left_columns, Some(m2m_table_alias)),
                (right_columns, Some(related_table_alias)),
            )
        };

        let m2m_join_data = rf
            .as_table(ctx)
            .alias(m2m_table_alias.to_string())
            .on(m2m_join_conditions);

        let aggregation_join_conditions = {
            let left_columns = vec![rf.related_field().m2m_column(ctx)];
            let right_columns = ModelProjection::from(rf.model().primary_identifier()).as_columns(ctx);
            build_join_conditions(
                (left_columns.into(), Some(m2m_table_alias)),
                (right_columns, Some(parent_alias)),
            )
        };

        Select::from_table(related_table)
            .value(count(asterisk()).alias(selection_name))
            .left_join(m2m_join_data)
            .and_where(aggregation_join_conditions)
            .with_filters(filter.clone(), Some(related_table_alias), ctx)
    }

    fn find_compatible_virtual_for_relation<'a>(
        &self,
        rs: &RelationSelection,
        mut parent_virtuals: impl Iterator<Item = &'a VirtualSelection>,
    ) -> Option<&'a VirtualSelection> {
        if rs.args.take.is_some() || rs.args.skip.is_some() || rs.args.cursor.is_some() || rs.args.distinct.is_some() {
            return None;
        }

        parent_virtuals.find(|vs| *vs.relation_field() == rs.field && vs.filter() == rs.args.filter.as_ref())
    }
}

pub(crate) trait SelectBuilderExt<'a> {
    fn with_filters(self, filter: Option<Filter>, parent_alias: Option<Alias>, ctx: &Context<'_>) -> Select<'a>;
    fn with_pagination(self, args: &QueryArguments, override_empty_take: Option<i64>) -> Select<'a>;
    fn with_ordering(self, args: &QueryArguments, parent_alias: Option<String>, ctx: &Context<'_>) -> Select<'a>;
    fn with_distinct(self, args: &QueryArguments, table_alias: Alias) -> Select<'a>;
    fn with_join_conditions(
        self,
        rf: &RelationField,
        left_alias: Alias,
        right_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a>;
    fn with_m2m_join_conditions(
        self,
        rf: &RelationField,
        left_alias: Alias,
        right_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a>;
    fn with_columns(self, columns: ColumnIterator) -> Select<'a>;
}

impl<'a> SelectBuilderExt<'a> for Select<'a> {
    fn with_filters(self, filter: Option<Filter>, parent_alias: Option<Alias>, ctx: &Context<'_>) -> Select<'a> {
        use crate::filter::*;

        if let Some(filter) = filter {
            let mut visitor = crate::filter::FilterVisitor::with_top_level_joins().set_parent_alias_opt(parent_alias);
            let (filter, joins) = visitor.visit_filter(filter, ctx);
            let select = self.and_where(filter);

            match joins {
                Some(joins) => joins.into_iter().fold(select, |acc, join| acc.join(join.data)),
                None => select,
            }
        } else {
            self
        }
    }

    fn with_pagination(self, args: &QueryArguments, override_empty_take: Option<i64>) -> Select<'a> {
        let take = match args.take.abs() {
            Some(_) if args.requires_inmemory_pagination(RelationLoadStrategy::Join) => override_empty_take,
            Some(take) => Some(take),
            None => override_empty_take,
        };

        let skip = match args.requires_inmemory_pagination(RelationLoadStrategy::Join) {
            true => None,
            false => args.skip,
        };

        let select = match take {
            Some(take) if !args.ignore_take => self.limit(take as usize),
            _ => self,
        };

        match skip {
            Some(skip) if !args.ignore_skip => select.offset(skip as usize),
            _ => select,
        }
    }

    fn with_ordering(self, args: &QueryArguments, parent_alias: Option<String>, ctx: &Context<'_>) -> Select<'a> {
        let order_by_definitions = OrderByBuilder::default()
            .with_parent_alias(parent_alias)
            .build(args, ctx);

        let select = order_by_definitions
            .iter()
            .flat_map(|j| &j.joins)
            .fold(self, |acc, join| acc.join(join.clone().data));

        order_by_definitions
            .iter()
            .fold(select, |acc, o| acc.order_by(o.order_definition.clone()))
    }

    fn with_distinct(self, args: &QueryArguments, table_alias: Alias) -> Select<'a> {
        if !args.can_distinct_in_db(RelationLoadStrategy::Join) {
            return self;
        }

        let Some(ref distinct) = args.distinct else { return self };

        let distinct_fields = distinct
            .scalars()
            .map(|sf| {
                Expression::from(Column::from((
                    table_alias.to_table_alias().to_string(),
                    sf.db_name().to_owned(),
                )))
            })
            .collect();

        self.distinct_on(distinct_fields)
    }

    fn with_join_conditions(
        self,
        rf: &RelationField,
        left_alias: Alias,
        right_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        self.and_where(rf.join_conditions(Some(left_alias), Some(right_alias), ctx))
    }

    fn with_m2m_join_conditions(
        self,
        rf: &RelationField,
        left_alias: Alias,
        right_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        self.and_where(rf.m2m_join_conditions(Some(left_alias), Some(right_alias), ctx))
    }

    fn with_columns(self, columns: ColumnIterator) -> Select<'a> {
        columns.into_iter().fold(self, |select, col| select.column(col))
    }
}

pub(crate) trait JoinConditionExt {
    fn join_conditions(
        &self,
        left_alias: Option<Alias>,
        right_alias: Option<Alias>,
        ctx: &Context<'_>,
    ) -> ConditionTree<'static>;
    fn m2m_join_conditions(
        &self,
        left_alias: Option<Alias>,
        right_alias: Option<Alias>,
        ctx: &Context<'_>,
    ) -> ConditionTree<'static>;
}

impl JoinConditionExt for RelationField {
    fn join_conditions(
        &self,
        left_alias: Option<Alias>,
        right_alias: Option<Alias>,
        ctx: &Context<'_>,
    ) -> ConditionTree<'static> {
        let left_columns = self.join_columns(ctx);
        let right_columns = ModelProjection::from(self.related_field().linking_fields()).as_columns(ctx);

        build_join_conditions((left_columns, left_alias), (right_columns, right_alias))
    }

    fn m2m_join_conditions(
        &self,
        left_alias: Option<Alias>,
        right_alias: Option<Alias>,
        ctx: &Context<'_>,
    ) -> ConditionTree<'static> {
        let left_columns = vec![self.m2m_column(ctx)];
        let right_columns = ModelProjection::from(self.related_model().primary_identifier()).as_columns(ctx);

        build_join_conditions((left_columns.into(), left_alias), (right_columns, right_alias))
    }
}

fn order_by_selection(rs: &RelationSelection) -> FieldSelection {
    let selection: Vec<_> = rs
        .args
        .order_by
        .iter()
        .flat_map(|order_by| match order_by {
            OrderBy::Scalar(x) => {
                // If the path is empty, the order by is done on the field itself in the outer select.
                if x.path.is_empty() {
                    vec![x.field.clone()]
                // If there are relations to traverse, select the linking fields of the first hop so that the outer select can perform a join to traverse the first relation.
                // This is necessary because the order by is done on a different join. The following hops are handled by the order by builder.
                } else {
                    first_hop_linking_fields(&x.path)
                }
            }
            OrderBy::Relevance(x) => x.fields.clone(),
            // Select the linking fields of the first hop so that the outer select can perform a join to traverse the relation.
            // This is necessary because the order by is done on a different join. The following hops are handled by the order by builder.
            OrderBy::ToManyAggregation(x) => first_hop_linking_fields(x.intermediary_hops()),
            OrderBy::ScalarAggregation(x) => vec![x.field.clone()],
        })
        .collect();

    FieldSelection::from(selection)
}

/// Returns the linking fields of the first hop in an order by path.
fn first_hop_linking_fields(hops: &[OrderByHop]) -> Vec<ScalarFieldRef> {
    hops.first()
        .and_then(|hop| hop.as_relation_hop())
        .map(|rf| rf.linking_fields().as_scalar_fields().unwrap())
        .unwrap_or_default()
}

fn relation_selection(rs: &RelationSelection) -> FieldSelection {
    let relation_fields = rs.relations().flat_map(|rs| join_fields(&rs.field)).collect::<Vec<_>>();

    FieldSelection::from(relation_fields)
}

fn filtering_selection(rs: &RelationSelection) -> FieldSelection {
    if let Some(filter) = &rs.args.filter {
        FieldSelection::from(extract_filter_scalars(filter))
    } else {
        FieldSelection::default()
    }
}

fn distinct_selection(rs: &RelationSelection) -> FieldSelection {
    rs.args.distinct.as_ref().cloned().unwrap_or_default()
}

fn json_obj_selections(rs: &RelationSelection) -> impl Iterator<Item = &SelectedField> + '_ {
    match rs.args.distinct.as_ref() {
        Some(distinct) if rs.args.requires_inmemory_distinct(RelationLoadStrategy::Join) => {
            Either::Left(rs.selections.iter().chain(distinct.selections()).unique())
        }
        _ => Either::Right(rs.selections.iter()),
    }
}

fn extract_filter_scalars(f: &Filter) -> Vec<ScalarFieldRef> {
    match f {
        Filter::And(x) => x.iter().flat_map(extract_filter_scalars).collect(),
        Filter::Or(x) => x.iter().flat_map(extract_filter_scalars).collect(),
        Filter::Not(x) => x.iter().flat_map(extract_filter_scalars).collect(),
        Filter::Scalar(x) => x.scalar_fields().into_iter().map(ToOwned::to_owned).collect(),
        Filter::ScalarList(x) => vec![x.field.clone()],
        Filter::OneRelationIsNull(x) => join_fields(&x.field),
        Filter::Relation(x) => join_fields(&x.field),
        _ => Vec::new(),
    }
}

fn join_fields(rf: &RelationField) -> Vec<ScalarFieldRef> {
    rf.linking_fields().as_scalar_fields().unwrap_or_default()
}

fn join_alias_name(rf: &RelationField) -> String {
    format!("{}_{}", rf.model().name(), rf.name())
}

fn m2m_join_alias_name(rf: &RelationField) -> String {
    format!("{}_{}_m2m", rf.model().name(), rf.name())
}

fn build_join_conditions(
    left: (ColumnIterator, Option<Alias>),
    right: (ColumnIterator, Option<Alias>),
) -> ConditionTree<'static> {
    let (left_columns, left_alias) = left;
    let (right_columns, right_alias) = right;

    left_columns
        .into_iter()
        .zip(right_columns)
        .fold(None::<ConditionTree>, |acc, (a, b)| {
            let a = a.opt_table(left_alias.map(|left| left.to_table_alias().to_string()));
            let b = b.opt_table(right_alias.map(|right| right.to_table_alias().to_string()));
            let condition = a.equals(b);

            match acc {
                Some(acc) => Some(acc.and(condition)),
                None => Some(condition.into()),
            }
        })
        .unwrap()
}

fn json_agg() -> Function<'static> {
    coalesce(vec![
        json_array_agg(Column::from(JSON_AGG_IDENT)).into(),
        Expression::from(Value::json(empty_json_array()).raw()),
    ])
    .alias(JSON_AGG_IDENT)
}

pub(crate) fn aliased_scalar_column(sf: &ScalarField, parent_alias: Alias, ctx: &Context<'_>) -> Column<'static> {
    let col = sf
        .as_column(ctx)
        .table(parent_alias.to_table_alias().to_string())
        .set_is_selected(true);

    if sf.name() != sf.db_name() {
        col.alias(sf.name().to_owned())
    } else {
        col
    }
}

#[inline]
fn empty_json_array() -> serde_json::Value {
    serde_json::Value::Array(Vec::new())
}

fn connector_flavour(args: &QueryArguments) -> Flavour {
    args.model().dm.schema.connector.flavour()
}

fn supports_lateral_join(args: &QueryArguments) -> bool {
    has_capability(args.model().dm.schema.connector, ConnectorCapability::LateralJoin)
}
