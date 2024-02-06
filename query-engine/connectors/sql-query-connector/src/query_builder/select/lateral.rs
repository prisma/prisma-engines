use super::*;

use crate::{
    context::Context,
    filter::alias::{Alias, AliasMode},
    model_extensions::AsColumn,
};

use std::collections::HashMap;

use quaint::ast::*;
use query_structure::*;

/// Select builder for joined queries. Relations are resolved using LATERAL JOINs.
#[derive(Debug, Default)]
pub(crate) struct LateralJoinSelectBuilder {
    alias: Alias,
    visited_virtuals: HashMap<VirtualSelection, Alias>,
}

impl JoinSelectBuilder for LateralJoinSelectBuilder {
    /// Builds a SELECT statement for the given query arguments and selected fields.
    ///
    /// ```sql
    /// SELECT
    ///   id,
    ///   name
    /// FROM "User"
    /// LEFT JOIN LATERAL (
    ///   SELECT JSON_OBJECT(<...>) FROM "Post" WHERE "Post"."authorId" = "User"."id
    /// ) as "post" ON TRUE
    /// ```
    fn build(&mut self, args: QueryArguments, selected_fields: &FieldSelection, ctx: &Context<'_>) -> Select<'static> {
        let (select, parent_alias) = self.build_default_select(&args, ctx);
        let select = self.with_relations(select, selected_fields.relations(), parent_alias, ctx);
        let select = self.with_virtual_selections(select, selected_fields.virtuals(), parent_alias, ctx);

        // Build selection as the last step utilizing the information we collected in
        // `with_relations` and `with_virtual_selections`.
        self.with_selection(select, selected_fields, parent_alias, ctx)
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
            SelectedField::Relation(rs) => {
                let table_name = match rs.field.relation().is_many_to_many() {
                    true => m2m_join_alias_name(&rs.field),
                    false => join_alias_name(&rs.field),
                };

                select.value(Column::from((table_name, JSON_AGG_IDENT)).alias(rs.field.name().to_owned()))
            }
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
        let (subselect, _) = self.build_to_one_select(
            rs,
            parent_alias,
            |expr: Expression<'_>| expr.alias(JSON_AGG_IDENT),
            true,
            ctx,
        );

        let join_table = Table::from(subselect).alias(join_alias_name(&rs.field));
        // LEFT JOIN LATERAL ( <join_table> ) AS <relation name> ON TRUE
        select.left_join(join_table.on(ConditionTree::single(true.raw())).lateral())
    }

    fn add_to_many_relation<'a>(
        &mut self,
        select: Select<'a>,
        rs: &RelationSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        let join_table_alias = join_alias_name(&rs.field);
        let join_table = Table::from(self.build_to_many_select(rs, parent_alias, ctx)).alias(join_table_alias);

        // LEFT JOIN LATERAL ( <join_table> ) AS <relation name> ON TRUE
        select.left_join(join_table.on(ConditionTree::single(true.raw())).lateral())
    }

    fn add_many_to_many_relation<'a>(
        &mut self,
        select: Select<'a>,
        rs: &RelationSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        let m2m_join = self.build_m2m_join(rs, parent_alias, ctx);

        select.left_join(m2m_join)
    }

    fn add_virtual_selection<'a>(
        &mut self,
        select: Select<'a>,
        vs: &VirtualSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        let alias = self.next_alias();
        let relation_count_select = self.build_virtual_select(vs, parent_alias, ctx);
        let table = Table::from(relation_count_select).alias(alias.to_table_string());

        // TODO: avoid cloning, consider using references as keys
        self.visited_virtuals.insert(vs.clone(), alias);

        select.left_join_lateral(table.on(ConditionTree::single(true.raw())))
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
            .filter_map(|field| match field {
                SelectedField::Scalar(sf) => Some((
                    Cow::from(sf.db_name().to_owned()),
                    Expression::from(sf.as_column(ctx).table(parent_alias.to_table_string())),
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
            .chain(self.build_json_obj_virtual_selection(rs.virtuals(), parent_alias, ctx))
            .collect();

        json_build_object(build_obj_params).into()
    }

    fn build_virtual_expr(
        &mut self,
        vs: &VirtualSelection,
        _parent_alias: Alias,
        _ctx: &Context<'_>,
    ) -> Expression<'static> {
        let virtual_selection_alias = self
            .visited_virtual_selection(vs)
            .expect("All virtual fields must be visited before calling build_virtual_expr");

        coalesce([
            Expression::from(Column::from((virtual_selection_alias.to_table_string(), vs.db_alias()))),
            Expression::from(0.raw()),
        ])
        .into()
    }

    fn next_alias(&mut self) -> Alias {
        self.alias = self.alias.inc(AliasMode::Table);
        self.alias
    }

    fn visited_virtual_selection(&self, vs: &VirtualSelection) -> Option<Alias> {
        self.visited_virtuals.get(vs).copied()
    }
}

impl LateralJoinSelectBuilder {
    fn build_m2m_join<'a>(&mut self, rs: &RelationSelection, parent_alias: Alias, ctx: &Context<'_>) -> JoinData<'a> {
        let rf = rs.field.clone();
        let m2m_table_alias = self.next_alias();
        let m2m_join_alias = self.next_alias();
        let outer_alias = self.next_alias();

        let m2m_join_data = Table::from(self.build_to_many_select(rs, m2m_table_alias, ctx))
            .alias(m2m_join_alias.to_table_string())
            .on(ConditionTree::single(true.raw()))
            .lateral();

        let child_table = rf.as_table(ctx).alias(m2m_table_alias.to_table_string());

        let inner = Select::from_table(child_table)
            .value(Column::from((m2m_join_alias.to_table_string(), JSON_AGG_IDENT)))
            .left_join(m2m_join_data) // join m2m table
            .with_m2m_join_conditions(&rf.related_field(), m2m_table_alias, parent_alias, ctx) // adds join condition to the child table
            // TODO: avoid clone filter
            .with_filters(rs.args.filter.clone(), Some(m2m_join_alias), ctx) // adds query filters
            .with_ordering(&rs.args, Some(m2m_join_alias.to_table_string()), ctx) // adds ordering stmts
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
}
