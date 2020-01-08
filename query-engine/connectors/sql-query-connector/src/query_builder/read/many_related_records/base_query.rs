use crate::{cursor_condition, filter_conversion::AliasedCondition};
use connector_interface::{OrderDirections, QueryArguments, SkipAndLimit};
use prisma_models::prelude::*;
use quaint::ast::{Aliasable, Column, Comparable, ConditionTree, Joinable, Select};

pub struct ManyRelatedRecordsBaseQuery<'a> {
    pub from_field: &'a RelationFieldRef,
    pub columns: Vec<Column<'static>>,
    pub from_record_ids: &'a [GraphqlId],
    pub query: Select<'a>,
    pub order_directions: OrderDirections,
    pub condition: ConditionTree<'a>,
    pub cursor: ConditionTree<'a>,
    pub window_limits: (i64, i64),
    pub skip_and_limit: SkipAndLimit,
}

impl<'a> ManyRelatedRecordsBaseQuery<'a> {
    pub fn new(
        from_field: &'a RelationFieldRef,
        from_record_ids: &'a [GraphqlId],
        query_arguments: QueryArguments,
        columns: Vec<Column<'static>>,
    ) -> ManyRelatedRecordsBaseQuery<'a> {
        // let cursor = cursor_condition::build(&query_arguments, from_field.related_model());
        // let window_limits = query_arguments.window_limits();
        // let skip_and_limit = query_arguments.skip_and_limit();

        // let order_directions = query_arguments.ordering_directions();
        // let condition = query_arguments
        //     .filter
        //     .map(|f| f.aliased_cond(None))
        //     .unwrap_or(ConditionTree::NoCondition);

        // let select = Select::from_table(from_field.related_model().as_table());

        // let query = if from_field.relation_is_inlined_in_child() {
        //     columns.iter().fold(select, |acc, col| acc.column(col.clone()))
        // } else {
        //     let join = from_field
        //         .relation()
        //         .as_table()
        //         .alias(Relation::TABLE_ALIAS)
        //         .on(from_field
        //             .related_model()
        //             .fields()
        //             .id()
        //             .as_column()
        //             .equals(from_field.opposite_column(true)));

        //     columns
        //         .iter()
        //         .fold(select, |acc, col| acc.column(col.clone()))
        //         .inner_join(join)
        // };

        // Self {
        //     from_field,
        //     columns,
        //     from_record_ids,
        //     query,
        //     order_directions,
        //     condition,
        //     cursor,
        //     window_limits,
        //     skip_and_limit,
        // }

        todo!()
    }
}
