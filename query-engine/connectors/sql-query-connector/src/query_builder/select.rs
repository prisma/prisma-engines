use std::borrow::Cow;

use crate::{
    context::Context,
    filter::alias::{Alias, AliasMode},
    model_extensions::{AsColumn, AsColumns, AsTable, RelationFieldExt},
    ordering::OrderByBuilder,
    sql_trace::SqlTraceComment,
};

use itertools::Itertools;
use quaint::prelude::*;
use query_structure::*;
use tracing::Span;

pub const JSON_AGG_IDENT: &str = "data";

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

        // SELECT ... FROM Table "t1"
        let select = Select::from_table(args.model().as_table(ctx).alias(table_alias.to_table_string()));

        // TODO: check how to select aggregated relations
        let select = selected_fields
            .selections()
            .fold(select, |acc, selection| match selection {
                SelectedField::Scalar(sf) => acc.column(sf.as_column(ctx).table(table_alias.to_table_string())),
                SelectedField::Relation(rs) => {
                    let table_name = match rs.field.relation().is_many_to_many() {
                        true => m2m_join_alias_name(&rs.field),
                        false => join_alias_name(&rs.field),
                    };

                    acc.value(Column::from((table_name, JSON_AGG_IDENT)).alias(rs.field.name().to_owned()))
                }
                _ => acc,
            });

        // Adds joins for relations
        let select = self.with_related_queries(select, selected_fields.relations(), table_alias, ctx);
        let select = self.with_ordering(select, &args, Some(table_alias.to_table_string()), ctx);
        let select = self.with_pagination(select, args.take_abs(), args.skip);
        let select = self.with_filters(select, args.filter, Some(table_alias), ctx);

        select.append_trace(&Span::current()).add_trace_id(ctx.trace_id)
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
            // LEFT JOIN LATERAL () AS <relation name> ON TRUE
            let join_select = Table::from(self.build_related_query_select(rs, parent_alias, ctx))
                .alias(join_alias_name(&rs.field))
                .on(ConditionTree::single(true.raw()))
                .lateral();

            select.left_join(join_select)
        }
    }

    fn build_related_query_select(
        &mut self,
        rs: &RelationSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'static> {
        let table_alias = self.next_alias();

        let build_obj_params = rs
            .selections
            .iter()
            .filter_map(|f| match f {
                SelectedField::Scalar(sf) => Some((
                    Cow::from(sf.db_name().to_owned()),
                    Expression::from(sf.as_column(ctx).table(table_alias.to_table_string())),
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
            .collect_vec();

        let inner_alias = join_alias_name(&rs.field.related_field());

        let related_table = rs
            .field
            .related_model()
            .as_table(ctx)
            .alias(table_alias.to_table_string());

        // SELECT JSON_BUILD_OBJECT()
        let inner = Select::from_table(related_table).value(json_build_object(build_obj_params).alias(JSON_AGG_IDENT));

        // WHERE parent.id = child.parent_id
        let inner = self.with_join_conditions(inner, &rs.field, parent_alias, table_alias, ctx);
        // LEFT JOIN LATERAL () AS <inner_alias> ON TRUE
        let inner = self.with_related_queries(inner, rs.relations(), table_alias, ctx);

        let linking_fields = rs.field.related_field().linking_fields();

        if rs.field.relation().is_many_to_many() {
            let order_by_selection = rs
                .args
                .order_by
                .iter()
                .flat_map(|order_by| match order_by {
                    OrderBy::Scalar(x) if x.path.is_empty() => vec![x.field.clone()],
                    OrderBy::Relevance(x) => x.fields.clone(),
                    _ => Vec::new(),
                })
                .collect_vec();
            let selection = FieldSelection::union(vec![FieldSelection::from(order_by_selection), linking_fields]);

            // SELECT <foreign_keys>
            // SELECT <orderby columns> ONLY if it's a m2m table as we need to order by outside of the inner select
            ModelProjection::from(selection)
                .as_columns(ctx)
                .fold(inner, |acc, c| acc.column(c.table(table_alias.to_table_string())))
        } else {
            // SELECT <foreign_keys>
            let inner = ModelProjection::from(linking_fields)
                .as_columns(ctx)
                .fold(inner, |acc, c| acc.column(c.table(table_alias.to_table_string())));

            let inner = self.with_ordering(inner, &rs.args, Some(table_alias.to_table_string()), ctx);
            let inner = self.with_pagination(inner, rs.args.take_abs(), rs.args.skip);
            let inner = self.with_filters(inner, rs.args.filter.clone(), Some(table_alias), ctx);

            let inner = Table::from(inner).alias(inner_alias.clone());
            let middle = Select::from_table(inner).column(Column::from((inner_alias.clone(), JSON_AGG_IDENT)));
            let outer = Select::from_table(Table::from(middle).alias(format!("{}_1", inner_alias))).value(json_agg());

            outer
        }
    }

    fn build_m2m_join<'a>(&mut self, rs: &RelationSelection, parent_alias: Alias, ctx: &Context<'_>) -> JoinData<'a> {
        let rf = rs.field.clone();
        let m2m_alias = m2m_join_alias_name(&rf);
        let m2m_table_alias = self.next_alias();

        let left_columns = rf.related_field().m2m_columns(ctx);
        let right_columns = ModelProjection::from(rf.model().primary_identifier()).as_columns(ctx);

        let conditions = left_columns
            .into_iter()
            .zip(right_columns)
            .fold(None::<ConditionTree>, |acc, (a, b)| {
                let a = a.table(m2m_table_alias.to_table_string());
                let b = b.table(parent_alias.to_table_string());
                let condition = a.equals(b);

                match acc {
                    Some(acc) => Some(acc.and(condition)),
                    None => Some(condition.into()),
                }
            })
            .unwrap();

        let inner = Select::from_table(rf.as_table(ctx).alias(m2m_table_alias.to_table_string()))
            .value(Column::from((join_alias_name(&rf), JSON_AGG_IDENT)))
            .and_where(conditions);

        let inner = self.with_ordering(inner, &rs.args, Some(join_alias_name(&rs.field)), ctx);
        let inner = self.with_pagination(inner, rs.args.take_abs(), rs.args.skip);
        // TODO: avoid clone?
        let inner = self.with_filters(inner, rs.args.filter.clone(), None, ctx);

        // TODO: parent_alias is likely wrong here
        let join_select = Table::from(self.build_related_query_select(rs, m2m_table_alias, ctx))
            .alias(join_alias_name(&rf))
            .on(ConditionTree::single(true.raw()))
            .lateral();

        let inner = inner.left_join(join_select);

        let outer = Select::from_table(Table::from(inner).alias(format!("{}_1", m2m_alias))).value(json_agg());

        Table::from(outer)
            .alias(m2m_alias)
            .on(ConditionTree::single(true.raw()))
            .lateral()
    }

    /// Builds the lateral join conditions
    fn with_join_conditions<'a>(
        &mut self,
        select: Select<'a>,
        rf: &RelationField,
        parent_alias: Alias,
        child_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        let join_columns = rf.join_columns(ctx);
        let related_join_columns = ModelProjection::from(rf.related_field().linking_fields()).as_columns(ctx);

        // WHERE Parent.id = Child.id
        let conditions = join_columns
            .zip(related_join_columns)
            .fold(None::<ConditionTree>, |acc, (a, b)| {
                let a = a.table(parent_alias.to_table_string());
                let b = b.table(child_alias.to_table_string());
                let condition = a.equals(b);

                match acc {
                    Some(acc) => Some(acc.and(condition)),
                    None => Some(condition.into()),
                }
            })
            .unwrap();

        select.and_where(conditions)
    }

    fn with_ordering<'a>(
        &mut self,
        select: Select<'a>,
        args: &QueryArguments,
        parent_alias: Option<String>,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        let order_by_definitions = OrderByBuilder::default()
            .with_parent_alias(parent_alias)
            .build(args, ctx);

        let select = order_by_definitions
            .iter()
            .flat_map(|j| &j.joins)
            .fold(select, |acc, join| acc.join(join.clone().data));

        order_by_definitions
            .iter()
            .fold(select, |acc, o| acc.order_by(o.order_definition.clone()))
    }

    fn with_pagination<'a>(&mut self, select: Select<'a>, take: Option<i64>, skip: Option<i64>) -> Select<'a> {
        let select = match take {
            Some(take) => select.limit(take as usize),
            None => select,
        };

        let select = match skip {
            Some(skip) => select.offset(skip as usize),
            None => select,
        };

        select
    }

    fn with_filters<'a>(
        &mut self,
        select: Select<'a>,
        filter: Option<Filter>,
        alias: Option<Alias>,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        use crate::filter::*;

        if let Some(filter) = filter {
            let mut visitor = crate::filter::FilterVisitor::with_top_level_joins().set_parent_alias_opt(alias);
            let (filter, joins) = visitor.visit_filter(filter, ctx);
            let select = select.and_where(filter);

            match joins {
                Some(joins) => joins.into_iter().fold(select, |acc, join| acc.join(join.data)),
                None => select,
            }
        } else {
            select
        }
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
