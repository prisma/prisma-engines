mod mysql;
mod postgres;

use std::borrow::Cow;

use psl::datamodel_connector::Flavour;
use tracing::Span;

use crate::{
    context::Context,
    filter::alias::Alias,
    model_extensions::{AsColumns, AsTable, ColumnIterator, RelationFieldExt},
    ordering::OrderByBuilder,
    sql_trace::SqlTraceComment,
};

use quaint::prelude::*;
use query_structure::*;

use self::{mysql::MysqlSelectBuilder, postgres::PostgresSelectBuilder};

pub(crate) const JSON_AGG_IDENT: &str = "__prisma_data__";

pub struct SelectBuilder {}

impl SelectBuilder {
    pub fn build(args: QueryArguments, selected_fields: &FieldSelection, ctx: &Context<'_>) -> Select<'static> {
        match connector_flavour(&args) {
            Flavour::Mysql => MysqlSelectBuilder::default().build(args, selected_fields, ctx),
            Flavour::Postgres | Flavour::Cockroach => {
                PostgresSelectBuilder::default().build(args, selected_fields, ctx)
            }
            _ => unreachable!("Connector does not support joined selects."),
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
    fn add_to_many_relation<'a>(
        &mut self,
        select: Select<'a>,
        rs: &RelationSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a>;
    /// Adds to `select` the SQL statements to fetch a m-n relation.
    fn add_many_to_many_relation<'a>(
        &mut self,
        select: Select<'a>,
        rs: &RelationSelection,
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
    /// Build the selection set for the `JSON_OBJECT` function.
    fn build_json_obj_selection(
        &mut self,
        field: &SelectedField,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Option<(String, Expression<'static>)>;
    /// Get the next alias for a table.
    fn next_alias(&mut self) -> Alias;

    fn with_selection<'a>(
        &mut self,
        select: Select<'a>,
        selected_fields: &FieldSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        selected_fields.selections().fold(select, |acc, selection| {
            self.build_selection(acc, selection, parent_alias, ctx)
        })
    }

    /// Builds the core select for a 1-1 relation.
    fn build_to_one_select(
        &mut self,
        rs: &RelationSelection,
        parent_alias: Alias,
        selection_modifier: impl FnOnce(Expression<'static>) -> Expression<'static>,
        ctx: &Context<'_>,
    ) -> (Select<'static>, Alias) {
        let rf = &rs.field;
        let child_table_alias = self.next_alias();
        let table = rs
            .field
            .related_field()
            .as_table(ctx)
            .alias(child_table_alias.to_table_string());
        let json_expr = self.build_json_obj_fn(rs, child_table_alias, ctx);

        let select = Select::from_table(table)
            .with_join_conditions(rf, parent_alias, child_table_alias, ctx)
            .with_filters(rs.args.filter.clone(), Some(child_table_alias), ctx)
            .value(selection_modifier(json_expr))
            .limit(1);

        (select, child_table_alias)
    }

    fn build_to_many_select(
        &mut self,
        rs: &RelationSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'static> {
        let inner_root_table_alias = self.next_alias();
        let root_alias = self.next_alias();
        let inner_alias = self.next_alias();
        let middle_alias = self.next_alias();

        let related_table = rs
            .related_model()
            .as_table(ctx)
            .alias(inner_root_table_alias.to_table_string());

        // SELECT * FROM "Table" as <table_alias> WHERE parent.id = child.parent_id
        let root = Select::from_table(related_table)
            .with_join_conditions(&rs.field, parent_alias, inner_root_table_alias, ctx)
            .comment("root select");

        // SELECT JSON_BUILD_OBJECT() FROM ( <root> )
        let inner = Select::from_table(Table::from(root).alias(root_alias.to_table_string()))
            .value(self.build_json_obj_fn(rs, root_alias, ctx).alias(JSON_AGG_IDENT));

        // LEFT JOIN LATERAL () AS <inner_alias> ON TRUE
        let inner = self.with_relations(inner, rs.relations(), root_alias, ctx);

        let linking_fields = rs.field.related_field().linking_fields();

        if rs.field.relation().is_many_to_many() {
            let selection: Vec<Column<'_>> =
                FieldSelection::union(vec![order_by_selection(rs), linking_fields, filtering_selection(rs)])
                    .into_projection()
                    .as_columns(ctx)
                    .map(|c| c.table(root_alias.to_table_string()))
                    .collect();

            // SELECT <foreign_keys>, <orderby columns>
            inner.with_columns(selection.into())
        } else {
            // select ordering, filtering & join fields from child selections to order, filter & join them on the outer query
            let inner_selection: Vec<Column<'_>> = FieldSelection::union(vec![
                order_by_selection(rs),
                filtering_selection(rs),
                relation_selection(rs),
            ])
            .into_projection()
            .as_columns(ctx)
            .map(|c| c.table(root_alias.to_table_string()))
            .collect();

            let inner = inner.with_columns(inner_selection.into()).comment("inner select");

            let middle_take = match connector_flavour(&rs.args) {
                // On MySQL, using LIMIT makes the ordering of the JSON_AGG working. Beware, this is undocumented behavior.
                // Note: Ideally, this should live in the MySQL select builder, but it's currently the only implementation difference
                // between MySQL and Postgres, so we keep it here for now to avoid code duplication.
                Flavour::Mysql => rs.args.take_abs().or(Some(i64::MAX)),
                _ => rs.args.take_abs(),
            };

            let middle = Select::from_table(Table::from(inner).alias(inner_alias.to_table_string()))
                // SELECT <inner_alias>.<JSON_ADD_IDENT>
                .column(Column::from((inner_alias.to_table_string(), JSON_AGG_IDENT)))
                // ORDER BY ...
                .with_ordering(&rs.args, Some(inner_alias.to_table_string()), ctx)
                // WHERE ...
                .with_filters(rs.args.filter.clone(), Some(inner_alias), ctx)
                // LIMIT $1 OFFSET $2
                .with_pagination(middle_take, rs.args.skip)
                .comment("middle select");

            // SELECT COALESCE(JSON_AGG(<inner_alias>), '[]') AS <inner_alias> FROM ( <middle> ) as <inner_alias_2>
            Select::from_table(Table::from(middle).alias(middle_alias.to_table_string()))
                .value(json_agg())
                .comment("outer select")
        }
    }

    fn build_json_obj_fn(
        &mut self,
        rs: &RelationSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Expression<'static> {
        let build_obj_params = rs
            .selections
            .iter()
            .filter_map(|f| {
                self.build_json_obj_selection(f, parent_alias, ctx)
                    .map(|(name, expr)| (Cow::from(name), expr))
            })
            .collect();

        json_build_object(build_obj_params).into()
    }

    fn with_relation<'a>(
        &mut self,
        select: Select<'a>,
        rs: &RelationSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        match (rs.field.is_list(), rs.field.relation().is_many_to_many()) {
            (true, true) => self.add_many_to_many_relation(select, rs, parent_alias, ctx),
            (true, false) => self.add_to_many_relation(select, rs, parent_alias, ctx),
            (false, _) => self.add_to_one_relation(select, rs, parent_alias, ctx),
        }
    }

    fn with_relations<'a, 'b>(
        &mut self,
        input: Select<'a>,
        relation_selections: impl Iterator<Item = &'b RelationSelection>,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        relation_selections.fold(input, |acc, rs| self.with_relation(acc, rs, parent_alias, ctx))
    }

    fn build_default_select(&mut self, args: &QueryArguments, ctx: &Context<'_>) -> (Select<'static>, Alias) {
        let table_alias = self.next_alias();
        let table = args.model().as_table(ctx).alias(table_alias.to_table_string());

        // SELECT ... FROM Table "t1"
        let select = Select::from_table(table)
            .with_ordering(args, Some(table_alias.to_table_string()), ctx)
            .with_filters(args.filter.clone(), Some(table_alias), ctx)
            .with_pagination(args.take_abs(), args.skip)
            .append_trace(&Span::current())
            .add_trace_id(ctx.trace_id);

        (select, table_alias)
    }
}

pub(crate) trait SelectBuilderExt<'a> {
    fn with_filters(self, filter: Option<Filter>, parent_alias: Option<Alias>, ctx: &Context<'_>) -> Select<'a>;
    fn with_pagination(self, take: Option<i64>, skip: Option<i64>) -> Select<'a>;
    fn with_ordering(self, args: &QueryArguments, parent_alias: Option<String>, ctx: &Context<'_>) -> Select<'a>;
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

    fn with_pagination(self, take: Option<i64>, skip: Option<i64>) -> Select<'a> {
        let select = match take {
            Some(take) => self.limit(take as usize),
            None => self,
        };

        let select = match skip {
            Some(skip) => select.offset(skip as usize),
            None => select,
        };

        select
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

fn order_by_selection(rs: &RelationSelection) -> FieldSelection {
    let selection: Vec<_> = rs
        .args
        .order_by
        .iter()
        .flat_map(|order_by| match order_by {
            OrderBy::Scalar(x) if x.path.is_empty() => vec![x.field.clone()],
            OrderBy::Relevance(x) => x.fields.clone(),
            _ => Vec::new(),
        })
        .collect();

    FieldSelection::from(selection)
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
    if rf.is_inlined_on_enclosing_model() {
        rf.scalar_fields()
    } else {
        rf.related_field().referenced_fields()
    }
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
            let a = a.opt_table(left_alias.map(|left| left.to_table_string()));
            let b = b.opt_table(right_alias.map(|right| right.to_table_string()));
            let condition = a.equals(b);

            match acc {
                Some(acc) => Some(acc.and(condition)),
                None => Some(condition.into()),
            }
        })
        .unwrap()
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
        let left_columns = self.m2m_columns(ctx);
        let right_columns = ModelProjection::from(self.related_model().primary_identifier()).as_columns(ctx);

        build_join_conditions((left_columns.into(), left_alias), (right_columns, right_alias))
    }
}

fn json_agg() -> Function<'static> {
    coalesce(vec![
        json_array_agg(Column::from(JSON_AGG_IDENT)).into(),
        Expression::from(Value::json(empty_json_array()).raw()),
    ])
    .alias(JSON_AGG_IDENT)
}

#[inline]
fn empty_json_array() -> serde_json::Value {
    serde_json::Value::Array(Vec::new())
}

fn connector_flavour(args: &QueryArguments) -> Flavour {
    args.model().dm.schema.connector.flavour()
}
