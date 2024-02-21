use super::*;

use crate::{
    context::Context,
    filter::alias::{Alias, AliasMode},
    model_extensions::AsColumn,
};

use std::collections::HashMap;

use quaint::ast::*;
use query_structure::*;

/// Represents a projection of a virtual field that is cheap to clone and compare but still has
/// enough information to determine whether it refers to the same field.
#[derive(PartialEq, Eq, Hash, Debug)]
enum VirtualSelectionKey {
    RelationCount(RelationField),
}

impl From<&VirtualSelection> for VirtualSelectionKey {
    fn from(vs: &VirtualSelection) -> Self {
        match vs {
            VirtualSelection::RelationCount(rf, _) => Self::RelationCount(rf.clone()),
        }
    }
}

/// Select builder for joined queries. Relations are resolved using LATERAL JOINs.
#[derive(Debug, Default)]
pub(crate) struct LateralJoinSelectBuilder {
    alias: Alias,
    visited_virtuals: HashMap<VirtualSelectionKey, String>,
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
        let select = self.with_relations(
            select,
            selected_fields.relations(),
            selected_fields.virtuals(),
            parent_alias,
            ctx,
        );
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
            SelectedField::Scalar(sf) => select.column(aliased_scalar_column(sf, parent_alias, ctx)),
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
        let (subselect, child_alias) = self.build_to_one_select(rs, parent_alias, ctx);

        let subselect = self.with_relations(subselect, rs.relations(), rs.virtuals(), child_alias, ctx);
        let subselect = self.with_virtual_selections(subselect, rs.virtuals(), child_alias, ctx);

        // Build the JSON object using the information we collected before in `with_relations` and
        // `with_virtual_selections`.
        let subselect = subselect.value(self.build_json_obj_fn(rs, child_alias, ctx).alias(JSON_AGG_IDENT));

        let join_table = Table::from(subselect).alias(join_alias_name(&rs.field));

        // LEFT JOIN LATERAL ( <join_table> ) AS <relation name> ON TRUE
        select.left_join(join_table.on(ConditionTree::single(true.raw())).lateral())
    }

    fn add_to_many_relation<'a, 'b>(
        &mut self,
        select: Select<'a>,
        rs: &RelationSelection,
        parent_virtuals: impl Iterator<Item = &'b VirtualSelection>,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        let join_table_alias = join_alias_name(&rs.field);
        let mut to_many_select = self.build_to_many_select(rs, parent_alias, ctx);

        if let Some(vs) = self.find_compatible_virtual_for_relation(rs, parent_virtuals) {
            self.visited_virtuals.insert(vs.into(), join_table_alias.clone());
            to_many_select = to_many_select.value(build_inline_virtual_selection(vs));
        }

        let join_table = Table::from(to_many_select).alias(join_table_alias);

        // LEFT JOIN LATERAL ( <join_table> ) AS <relation name> ON TRUE
        select.left_join(join_table.on(ConditionTree::single(true.raw())).lateral())
    }

    fn add_many_to_many_relation<'a, 'b>(
        &mut self,
        select: Select<'a>,
        rs: &RelationSelection,
        parent_virtuals: impl Iterator<Item = &'b VirtualSelection>,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Select<'a> {
        let m2m_join = self.build_m2m_join(rs, parent_virtuals, parent_alias, ctx);

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

        self.visited_virtuals.insert(vs.into(), alias.to_table_string());

        select.left_join_lateral(table.on(ConditionTree::single(true.raw())))
    }

    fn build_json_obj_fn(
        &mut self,
        rs: &RelationSelection,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> Expression<'static> {
        let build_obj_params = json_obj_selections(rs)
            .filter_map(|field| match field {
                SelectedField::Scalar(sf) => Some((
                    Cow::from(sf.name().to_owned()),
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
            .visited_virtuals
            .remove(&vs.into())
            .expect("All virtual fields must be visited before calling build_virtual_expr");

        coalesce([
            Expression::from(Column::from((virtual_selection_alias, vs.db_alias()))),
            Expression::from(0.raw()),
        ])
        .into()
    }

    fn next_alias(&mut self) -> Alias {
        self.alias = self.alias.inc(AliasMode::Table);
        self.alias
    }

    fn was_virtual_processed_in_relation(&self, vs: &VirtualSelection) -> bool {
        self.visited_virtuals.contains_key(&vs.into())
    }
}

impl LateralJoinSelectBuilder {
    fn build_m2m_join<'a, 'b>(
        &mut self,
        rs: &RelationSelection,
        parent_virtuals: impl Iterator<Item = &'b VirtualSelection>,
        parent_alias: Alias,
        ctx: &Context<'_>,
    ) -> JoinData<'a> {
        let rf = rs.field.clone();
        let m2m_table_alias = self.next_alias();
        let m2m_join_alias = self.next_alias();
        let outer_alias = self.next_alias();
        let json_data_alias = m2m_join_alias_name(&rf);

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
            .with_distinct(&rs.args, m2m_join_alias)
            .with_ordering(&rs.args, Some(m2m_join_alias.to_table_string()), ctx) // adds ordering stmts
            .with_pagination(&rs.args, None)
            .comment("inner"); // adds pagination

        let mut outer = Select::from_table(Table::from(inner).alias(outer_alias.to_table_string()))
            .value(json_agg())
            .comment("outer");

        if let Some(vs) = self.find_compatible_virtual_for_relation(rs, parent_virtuals) {
            self.visited_virtuals.insert(vs.into(), json_data_alias.clone());
            outer = outer.value(build_inline_virtual_selection(vs));
        }

        Table::from(outer)
            .alias(json_data_alias)
            .on(ConditionTree::single(true.raw()))
            .lateral()
    }
}

fn build_inline_virtual_selection<'a>(vs: &VirtualSelection) -> Expression<'a> {
    match vs {
        VirtualSelection::RelationCount(..) => count(Column::from(JSON_AGG_IDENT)).alias(vs.db_alias()).into(),
    }
}
