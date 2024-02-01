use std::{borrow::Cow, collections::BTreeMap};
use tracing::Span;

use crate::{
    context::Context,
    filter::alias::{Alias, AliasMode},
    model_extensions::{AsColumn, AsColumns, AsTable, ColumnIterator, RelationFieldExt},
    ordering::OrderByBuilder,
    sql_trace::SqlTraceComment,
};

use quaint::prelude::*;
use query_structure::*;

pub const JSON_AGG_IDENT: &str = "__prisma_data__";

#[derive(Debug, Default)]
pub(crate) struct SelectBuilder {
    alias: Alias,
}

impl SelectBuilder {
    pub(crate) fn next_alias(&mut self) -> Alias {
        self.alias = self.alias.inc(AliasMode::Table);
        self.alias
    }

    pub(crate) fn build(
        &mut self,
        args: QueryArguments,
        selected_fields: &FieldSelection,
        ctx: &Context<'_>,
    ) -> Select<'static> {
        let table_alias = self.next_alias();
        let table = args.model().as_table(ctx).alias(table_alias.to_table_string());

        // SELECT ... FROM Table "t1"
        let select = Select::from_table(table)
            .with_selection(selected_fields, table_alias, ctx)
            .with_ordering(&args, Some(table_alias.to_table_string()), ctx)
            .with_pagination(args.take_abs(), args.skip)
            .with_filters(args.filter, Some(table_alias), ctx)
            .append_trace(&Span::current())
            .add_trace_id(ctx.trace_id);

        // Adds joins for relations
        let select = self.with_related_queries(select, selected_fields.relations(), table_alias, ctx);

        // Adds joins for relation aggregations. Other potential future kinds of virtual fields
        // might or might not require joins and might be processed differently.
        self.with_relation_aggregation_queries(select, selected_fields.virtuals(), table_alias, ctx)
    }

    fn with_related_queries<'a, 'b>(
        &mut self,
        input: Select<'a>,
        relation_selections: impl Iterator<Item = &'b RelationSelection>,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        relation_selections.fold(input, |acc, rs| self.with_related_query(acc, rs, parent_alias, ctx))
    }

    fn with_related_query<'a>(
        &mut self,
        select: Select<'a>,
        rs: &RelationSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        if rs.field.relation().is_many_to_many() {
            // m2m relations need to left join on the relation table first
            let m2m_join = self.build_m2m_join(rs, parent_alias, ctx);

            select.left_join(m2m_join)
        } else {
            let join_table_alias = join_alias_name(&rs.field);
            let join_table =
                Table::from(self.build_related_query_select(rs, parent_alias, ctx)).alias(join_table_alias);

            // LEFT JOIN LATERAL ( <join_table> ) AS <relation name> ON TRUE
            select.left_join(join_table.on(ConditionTree::single(true.raw())).lateral())
        }
    }

    fn build_related_query_select(
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
            .value(build_json_obj_fn(rs, ctx, root_alias).alias(JSON_AGG_IDENT));

        // LEFT JOIN LATERAL () AS <inner_alias> ON TRUE
        let inner = self.with_related_queries(inner, rs.relations(), root_alias, ctx);

        // LEFT JOIN LATERAL ( <relation aggregation query> ) ON TRUE
        let inner = self.with_relation_aggregation_queries(inner, rs.virtuals(), root_alias, ctx);

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

            let middle = Select::from_table(Table::from(inner).alias(inner_alias.to_table_string()))
                // SELECT <inner_alias>.<JSON_ADD_IDENT>
                .column(Column::from((inner_alias.to_table_string(), JSON_AGG_IDENT)))
                // ORDER BY ...
                .with_ordering(&rs.args, Some(inner_alias.to_table_string()), ctx)
                // WHERE ...
                .with_filters(rs.args.filter.clone(), Some(inner_alias), ctx)
                // LIMIT $1 OFFSET $2
                .with_pagination(rs.args.take_abs(), rs.args.skip)
                .comment("middle select");

            // SELECT COALESCE(JSON_AGG(<inner_alias>), '[]') AS <inner_alias> FROM ( <middle> ) as <inner_alias_2>
            Select::from_table(Table::from(middle).alias(middle_alias.to_table_string()))
                .value(json_agg())
                .comment("outer select")
        }
    }

    fn build_m2m_join<'a>(&mut self, rs: &RelationSelection, parent_alias: Alias, ctx: &Context<'_>) -> JoinData<'a> {
        let rf = rs.field.clone();
        let m2m_table_alias = self.next_alias();
        let m2m_join_alias = self.next_alias();
        let outer_alias = self.next_alias();

        let left_columns = rf.related_field().m2m_columns(ctx);
        let right_columns = ModelProjection::from(rf.model().primary_identifier()).as_columns(ctx);

        let join_conditions =
            build_join_conditions((left_columns.into(), m2m_table_alias), (right_columns, parent_alias));

        let m2m_join_data = Table::from(self.build_related_query_select(rs, m2m_table_alias, ctx))
            .alias(m2m_join_alias.to_table_string())
            .on(ConditionTree::single(true.raw()))
            .lateral();

        let child_table = rf.as_table(ctx).alias(m2m_table_alias.to_table_string());

        let inner = Select::from_table(child_table)
            .value(Column::from((m2m_join_alias.to_table_string(), JSON_AGG_IDENT)))
            .left_join(m2m_join_data) // join m2m table
            .and_where(join_conditions) // adds join condition to the child table
            .with_ordering(&rs.args, Some(m2m_join_alias.to_table_string()), ctx) // adds ordering stmts
            .with_filters(rs.args.filter.clone(), Some(m2m_join_alias), ctx) // adds query filters // TODO: avoid clone filter
            .with_pagination(rs.args.take_abs(), rs.args.skip)
            .comment("inner"); // adds pagination

        let outer = Select::from_table(Table::from(inner).alias(outer_alias.to_table_string()))
            .value(json_agg())
            .comment("outer");

        Table::from(outer)
            .alias(m2m_join_alias_name(&rf))
            .on(ConditionTree::single(true.raw()))
            .lateral()
    }

    fn with_relation_aggregation_queries<'a, 'b>(
        &mut self,
        select: Select<'a>,
        selections: impl Iterator<Item = &'b VirtualSelection>,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        selections.fold(select, |acc, vs| {
            self.with_relation_aggregation_query(acc, vs, parent_alias, ctx)
        })
    }

    fn with_relation_aggregation_query<'a>(
        &mut self,
        select: Select<'a>,
        vs: &VirtualSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        match vs {
            VirtualSelection::RelationCount(rf, filter) => {
                let table_alias = relation_count_alias_name(rf);

                let relation_count_select = if rf.relation().is_many_to_many() {
                    self.build_relation_count_query_m2m(vs.db_alias(), rf, filter, parent_alias, ctx)
                } else {
                    self.build_relation_count_query(vs.db_alias(), rf, filter, parent_alias, ctx)
                };

                let table = Table::from(relation_count_select).alias(table_alias);

                select.left_join_lateral(table.on(ConditionTree::single(true.raw())))
            }
        }
    }

    fn build_relation_count_query<'a>(
        &mut self,
        selection_name: impl Into<Cow<'static, str>>,
        rf: &RelationField,
        filter: &Option<Filter>,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        let related_table_alias = self.next_alias();

        let related_table = rf
            .related_model()
            .as_table(ctx)
            .alias(related_table_alias.to_table_string());

        let select = Select::from_table(related_table)
            .value(count(asterisk()).alias(selection_name))
            .with_join_conditions(rf, parent_alias, related_table_alias, ctx)
            .with_filters(filter.clone(), Some(related_table_alias), ctx);

        select
    }

    fn build_relation_count_query_m2m<'a>(
        &mut self,
        selection_name: impl Into<Cow<'static, str>>,
        rf: &RelationField,
        filter: &Option<Filter>,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        let related_table_alias = self.next_alias();
        let m2m_table_alias = self.next_alias();

        let related_table = rf
            .related_model()
            .as_table(ctx)
            .alias(related_table_alias.to_table_string());

        let m2m_join_conditions = {
            let left_columns = rf.join_columns(ctx);
            let right_columns = ModelProjection::from(rf.related_field().linking_fields()).as_columns(ctx);
            build_join_conditions((left_columns, m2m_table_alias), (right_columns, related_table_alias))
        };

        let m2m_join_data = rf
            .as_table(ctx)
            .alias(m2m_table_alias.to_table_string())
            .on(m2m_join_conditions);

        let aggregation_join_conditions = {
            let left_columns = rf.related_field().m2m_columns(ctx);
            let right_columns = ModelProjection::from(rf.model().primary_identifier()).as_columns(ctx);
            build_join_conditions((left_columns.into(), m2m_table_alias), (right_columns, parent_alias))
        };

        let select = Select::from_table(related_table)
            .value(count(asterisk()).alias(selection_name))
            .left_join(m2m_join_data)
            .and_where(aggregation_join_conditions)
            .with_filters(filter.clone(), Some(related_table_alias), ctx);

        select
    }
}

trait SelectBuilderExt<'a> {
    fn with_filters(self, filter: Option<Filter>, parent_alias: Option<Alias>, ctx: &Context<'_>) -> Select<'a>;
    fn with_pagination(self, take: Option<i64>, skip: Option<i64>) -> Select<'a>;
    fn with_ordering(self, args: &QueryArguments, parent_alias: Option<String>, ctx: &Context<'_>) -> Select<'a>;
    fn with_join_conditions(
        self,
        rf: &RelationField,
        parent_alias: Alias,
        child_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a>;
    fn with_selection(self, selected_fields: &FieldSelection, table_alias: Alias, ctx: &Context<'_>) -> Select<'a>;
    fn with_virtuals_from_selection(self, selected_fields: &FieldSelection) -> Select<'a>;
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
        parent_alias: Alias,
        child_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        let join_columns = rf.join_columns(ctx);
        let related_join_columns = ModelProjection::from(rf.related_field().linking_fields()).as_columns(ctx);

        let conditions = build_join_conditions((join_columns, parent_alias), (related_join_columns, child_alias));

        // WHERE Parent.id = Child.id
        self.and_where(conditions)
    }

    fn with_selection(self, selected_fields: &FieldSelection, table_alias: Alias, ctx: &Context<'_>) -> Select<'a> {
        selected_fields
            .selections()
            .fold(self, |acc, selection| match selection {
                SelectedField::Scalar(sf) => acc.column(
                    sf.as_column(ctx)
                        .table(table_alias.to_table_string())
                        .set_is_selected(true),
                ),
                SelectedField::Relation(rs) => {
                    let table_name = match rs.field.relation().is_many_to_many() {
                        true => m2m_join_alias_name(&rs.field),
                        false => join_alias_name(&rs.field),
                    };

                    acc.value(Column::from((table_name, JSON_AGG_IDENT)).alias(rs.field.name().to_owned()))
                }
                _ => acc,
            })
            .with_virtuals_from_selection(selected_fields)
    }

    fn with_virtuals_from_selection(self, selected_fields: &FieldSelection) -> Select<'a> {
        build_virtual_selection(selected_fields.virtuals())
            .into_iter()
            .fold(self, |select, (alias, expr)| select.value(expr.alias(alias)))
    }

    fn with_columns(self, columns: ColumnIterator) -> Select<'a> {
        columns.into_iter().fold(self, |select, col| select.column(col))
    }
}

fn build_join_conditions(
    (left_columns, left_alias): (ColumnIterator, Alias),
    (right_columns, right_alias): (ColumnIterator, Alias),
) -> ConditionTree<'static> {
    left_columns
        .zip(right_columns)
        .fold(None::<ConditionTree>, |acc, (a, b)| {
            let a = a.table(left_alias.to_table_string());
            let b = b.table(right_alias.to_table_string());
            let condition = a.equals(b);

            match acc {
                Some(acc) => Some(acc.and(condition)),
                None => Some(condition.into()),
            }
        })
        .unwrap()
}

fn build_json_obj_fn(rs: &RelationSelection, ctx: &Context<'_>, root_alias: Alias) -> Function<'static> {
    let build_obj_params = rs
        .selections
        .iter()
        .filter_map(|f| match f {
            SelectedField::Scalar(sf) => Some((
                Cow::from(sf.db_name().to_owned()),
                Expression::from(sf.as_column(ctx).table(root_alias.to_table_string())),
            )),
            SelectedField::Relation(rs) => {
                let table_name = match rs.field.relation().is_many_to_many() {
                    true => m2m_join_alias_name(&rs.field),
                    false => join_alias_name(&rs.field),
                };

                Some((
                    Cow::from(rs.field.name().to_owned()),
                    Expression::from(Column::from((table_name, JSON_AGG_IDENT))),
                ))
            }
            _ => None,
        })
        .chain(build_virtual_selection(rs.virtuals()))
        .collect();

    json_build_object(build_obj_params)
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

fn json_agg() -> Function<'static> {
    coalesce(vec![
        json_array_agg(Column::from(JSON_AGG_IDENT)).into(),
        Expression::from("[]".raw()),
    ])
    .alias(JSON_AGG_IDENT)
}

fn build_virtual_selection<'a>(
    virtual_fields: impl Iterator<Item = &'a VirtualSelection>,
) -> Vec<(Cow<'static, str>, Expression<'static>)> {
    let mut selected_objects = BTreeMap::new();

    for vs in virtual_fields {
        match vs {
            VirtualSelection::RelationCount(rf, _) => {
                let (object_name, field_name) = vs.serialized_name();

                let coalesce_args: Vec<Expression<'static>> = vec![
                    Column::from((relation_count_alias_name(rf), vs.db_alias())).into(),
                    0.raw().into(),
                ];

                selected_objects
                    .entry(object_name)
                    .or_insert(Vec::new())
                    .push((field_name.to_owned().into(), coalesce(coalesce_args).into()));
            }
        }
    }

    selected_objects
        .into_iter()
        .map(|(name, fields)| (name.into(), json_build_object(fields).into()))
        .collect()
}

fn relation_count_alias_name(rf: &RelationField) -> String {
    format!("aggr_count_{}_{}", rf.model().name(), rf.name())
}
