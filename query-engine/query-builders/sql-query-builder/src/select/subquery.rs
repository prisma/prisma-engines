use super::*;

use crate::{context::Context, filter::alias::Alias, model_extensions::*};

use quaint::ast::*;
use query_structure::*;

/// Select builder for joined queries. Relations are resolved using correlated sub-queries.
#[derive(Debug, Default)]
pub(crate) struct SubqueriesSelectBuilder;

impl JoinSelectBuilder for SubqueriesSelectBuilder {
    /// Builds a SELECT statement for the given query arguments and selected fields.
    ///
    /// ```sql
    /// SELECT
    ///   id,
    ///   name,
    ///   (
    ///     SELECT JSON_OBJECT(<...>) FROM "Post" WHERE "Post"."authorId" = "User"."id
    ///   ) as `post`
    /// FROM "User"
    /// ```
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
            SelectedField::Scalar(sf) => select.column(aliased_scalar_column(sf, parent_alias, ctx)),
            SelectedField::Relation(rs) => self.with_relation(select, rs, Vec::new().iter(), parent_alias, ctx),
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
        let (subselect, child_alias) = self.build_to_one_select(rs, parent_alias, ctx);
        let subselect = subselect.value(self.build_json_obj_fn(rs, child_alias, ctx));

        select.value(Expression::from(subselect).alias(rs.field.name().to_owned()))
    }

    fn add_to_many_relation<'a, 'b>(
        &mut self,
        select: Select<'a>,
        rs: &RelationSelection,
        _parent_virtuals: impl Iterator<Item = &'b VirtualSelection>,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        let subselect = self.build_to_many_select(rs, parent_alias, ctx);

        select.value(Expression::from(subselect).alias(rs.field.name().to_owned()))
    }

    fn add_many_to_many_relation<'a, 'b>(
        &mut self,
        select: Select<'a>,
        rs: &RelationSelection,
        _parent_virtuals: impl Iterator<Item = &'b VirtualSelection>,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        let subselect = self.build_m2m_select(rs, parent_alias, ctx);

        select.value(Expression::from(subselect).alias(rs.field.name().to_owned()))
    }

    fn add_virtual_selection<'a>(
        &mut self,
        select: Select<'a>,
        vs: &VirtualSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        let virtual_select = self.build_virtual_select(vs, parent_alias, ctx);
        let alias = relation_count_alias_name(vs.relation_field());

        select.value(Expression::from(virtual_select).alias(alias))
    }

    fn build_json_obj_fn(
        &mut self,
        rs: &RelationSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Expression<'static> {
        let virtuals = self.build_json_obj_virtual_selection(rs.virtuals(), parent_alias, ctx);
        let build_obj_params = json_obj_selections(rs)
            .filter_map(|field| match field {
                SelectedField::Scalar(sf) => Some((
                    Cow::from(sf.name().to_owned()),
                    Expression::from(sf.as_column(ctx).table(parent_alias.to_table_alias().to_string())),
                )),
                SelectedField::Relation(rs) => Some((
                    Cow::from(rs.field.name().to_owned()),
                    Expression::from(self.with_relation(Select::default(), rs, Vec::new().iter(), parent_alias, ctx)),
                )),
                _ => None,
            })
            .chain(virtuals)
            .collect();

        json_build_object(build_obj_params).into()
    }

    fn build_virtual_expr(
        &mut self,
        vs: &VirtualSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Expression<'static> {
        coalesce([
            Expression::from(self.build_virtual_select(vs, parent_alias, ctx)),
            Expression::from(0.raw()),
        ])
        .into()
    }

    fn was_virtual_processed_in_relation(&self, _vs: &VirtualSelection) -> bool {
        false
    }
}

impl SubqueriesSelectBuilder {
    fn build_m2m_select<'a>(&mut self, rs: &RelationSelection, parent_alias: Alias, ctx: &Context<'_>) -> Select<'a> {
        let rf = rs.field.clone();
        let m2m_table_alias = ctx.next_table_alias();
        let root_alias = ctx.next_table_alias();
        let outer_alias = ctx.next_table_alias();

        let m2m_join_data =
            rf.related_model()
                .as_table(ctx)
                .on(rf.m2m_join_conditions(Some(m2m_table_alias), None, ctx));

        let m2m_table = rf.as_table(ctx).alias(m2m_table_alias.to_string());

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
        let override_empty_take = (!rs.args.order_by.is_empty()).then_some(i64::MAX);

        let inner = Select::from_table(Table::from(root).alias(root_alias.to_string()))
            .value(self.build_json_obj_fn(rs, root_alias, ctx).alias(JSON_AGG_IDENT))
            .with_pagination(&rs.args, override_empty_take)
            .comment("inner"); // adds pagination

        Select::from_table(Table::from(inner).alias(outer_alias.to_string()))
            .value(json_agg())
            .comment("outer")
    }
}

fn relation_count_alias_name(rf: &RelationField) -> String {
    format!("aggr_count_{}_{}", rf.model().name(), rf.name())
}
