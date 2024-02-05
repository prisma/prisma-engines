use super::*;
use crate::{
    context::Context,
    filter::alias::{Alias, AliasMode},
    model_extensions::AsColumn,
};

use quaint::ast::*;
use query_structure::*;

#[derive(Debug, Default)]
pub(crate) struct PostgresSelectBuilder {
    alias: Alias,
}

impl PostgresSelectBuilder {}

impl JoinSelectBuilder for PostgresSelectBuilder {
    fn build(&mut self, args: QueryArguments, selected_fields: &FieldSelection, ctx: &Context<'_>) -> Select<'static> {
        let (select, parent_alias) = self.build_default_select(&args, ctx);
        let select = self.with_selection(select, selected_fields, parent_alias, ctx);

        self.with_relations(select, selected_fields.relations(), parent_alias, ctx)
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
        let (subselect, child_alias) =
            self.build_to_one_select(rs, parent_alias, |expr: Expression<'_>| expr.alias(JSON_AGG_IDENT), ctx);
        let subselect = self.with_relations(subselect, rs.relations(), child_alias, ctx);

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
        // m2m relations need to left join on the relation table first
        let m2m_join = self.build_m2m_join(rs, parent_alias, ctx);

        select.left_join(m2m_join)
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
            SelectedField::Relation(rs) => {
                let table_name = match rs.field.relation().is_many_to_many() {
                    true => m2m_join_alias_name(&rs.field),
                    false => join_alias_name(&rs.field),
                };

                Some((
                    rs.field.name().to_owned(),
                    Expression::from(Column::from((table_name, JSON_AGG_IDENT))),
                ))
            }
            _ => None,
        }
    }

    fn next_alias(&mut self) -> Alias {
        self.alias = self.alias.inc(AliasMode::Table);
        self.alias
    }
}

impl PostgresSelectBuilder {
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

        let m2m_join_data = Table::from(self.build_to_many_select(rs, m2m_table_alias, ctx))
            .alias(m2m_join_alias.to_table_string())
            .on(ConditionTree::single(true.raw()))
            .lateral();

        let child_table = rf.as_table(ctx).alias(m2m_table_alias.to_table_string());

        let inner = Select::from_table(child_table)
            .value(Column::from((m2m_join_alias.to_table_string(), JSON_AGG_IDENT)))
            .left_join(m2m_join_data) // join m2m table
            .with_m2m_join_conditions(&rf.related_field(), m2m_table_alias, parent_alias, ctx) // adds join condition to the child table
            .with_filters(rs.args.filter.clone(), Some(m2m_join_alias), ctx) // adds query filters // TODO: avoid clone filter
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
