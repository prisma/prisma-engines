use super::*;
use crate::{
    context::Context,
    filter::alias::{Alias, AliasMode},
    model_extensions::*,
};

use quaint::ast::*;
use query_structure::*;

#[derive(Debug, Default)]
pub(crate) struct MysqlSelectBuilder {
    alias: Alias,
}

impl JoinSelectBuilder for MysqlSelectBuilder {
    fn build(&mut self, args: QueryArguments, selected_fields: &FieldSelection, ctx: &Context<'_>) -> Select<'static> {
        let (select, alias) = self.build_default_select(&args, ctx);

        self.with_selection(select, selected_fields, alias, ctx)
    }

    fn build_selection<'a>(
        &mut self,
        select: Select<'a>,
        field: &SelectedField,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        match field {
            SelectedField::Scalar(sf) => select.column(
                sf.as_column(ctx)
                    .table(parent_alias.to_table_string())
                    .set_is_selected(true),
            ),
            SelectedField::Relation(rs) => self.with_relation(select, rs, parent_alias, ctx),
            _ => select,
        }
    }

    fn add_to_one_relation<'a>(
        &mut self,
        select: Select<'a>,
        rs: &RelationSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        let (subselect, _) = self.build_to_one_select(rs, parent_alias, |x| x, ctx);

        select.value(Expression::from(subselect).alias(rs.field.name().to_owned()))
    }

    fn add_to_many_relation<'a>(
        &mut self,
        select: Select<'a>,
        rs: &query_structure::prelude::RelationSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        let join_table =
            Expression::from(self.build_to_many_select(rs, parent_alias, ctx)).alias(rs.field.name().to_owned());

        select.value(join_table)
    }

    fn add_many_to_many_relation<'a>(
        &mut self,
        select: Select<'a>,
        rs: &query_structure::prelude::RelationSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        let m2m_select = self.build_m2m_select(rs, parent_alias, ctx);

        select.value(Expression::from(m2m_select).alias(rs.field.name().to_owned()))
    }

    fn build_json_obj_selection(
        &mut self,
        field: &SelectedField,

        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Option<(String, Expression<'static>)> {
        match field {
            SelectedField::Scalar(sf) => Some((
                sf.db_name().to_owned(),
                Expression::from(sf.as_column(ctx).table(parent_alias.to_table_string())),
            )),
            SelectedField::Relation(rs) => Some((
                rs.field.name().to_owned(),
                Expression::from(self.with_relation(Select::default(), rs, parent_alias, ctx)),
            )),
            _ => None,
        }
    }

    fn next_alias(&mut self) -> Alias {
        self.alias = self.alias.inc(AliasMode::Table);
        self.alias
    }
}

impl MysqlSelectBuilder {
    fn build_m2m_select<'a>(&mut self, rs: &RelationSelection, parent_alias: Alias, ctx: &Context<'_>) -> Select<'a> {
        let rf = rs.field.clone();
        let m2m_table_alias = self.next_alias();
        let root_alias = self.next_alias();
        let outer_alias = self.next_alias();

        let m2m_join_data =
            rf.related_model()
                .as_table(ctx)
                .on(rf.m2m_join_conditions(Some(m2m_table_alias), None, ctx));

        let m2m_table = rf.as_table(ctx).alias(m2m_table_alias.to_table_string());

        let root = Select::from_table(m2m_table)
            .inner_join(m2m_join_data)
            .value(rf.related_model().as_table(ctx).asterisk())
            .with_ordering(&rs.args, None, ctx) // adds ordering stmts
            // Keep join conditions _before_ user filters to ensure index is used first
            .and_where(
                rf.related_field()
                    .m2m_join_conditions(Some(m2m_table_alias), Some(parent_alias), ctx),
            ) // adds join condition to the child table
            .with_filters(rs.args.filter.clone(), None, ctx) // adds query filters
            .comment("root");

        // On MySQL, using LIMIT makes the ordering of the JSON_AGG working. Beware, this is undocumented behavior.
        let take = match rs.args.order_by.is_empty() {
            true => rs.args.take_abs(),
            false => rs.args.take_abs().or(Some(i64::MAX)),
        };

        let inner = Select::from_table(Table::from(root).alias(root_alias.to_table_string()))
            .value(self.build_json_obj_fn(rs, root_alias, ctx).alias(JSON_AGG_IDENT))
            .with_pagination(take, rs.args.skip)
            .comment("inner"); // adds pagination

        Select::from_table(Table::from(inner).alias(outer_alias.to_table_string()))
            .value(json_agg())
            .comment("outer")
    }
}
