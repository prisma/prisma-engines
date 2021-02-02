use crate::query_arguments_ext::QueryArgumentsExt;
use connector_interface::QueryArguments;
use prisma_models::*;
use quaint::ast::*;

/// Builds all expressions for an `ORDER BY` clause based on the query arguments.
pub fn build(
    query_arguments: &QueryArguments,
    base_model: &ModelRef, // The model the ordering will start from
) -> (Vec<OrderDefinition<'static>>, Vec<JoinData<'static>>) {
    let needs_reversed_order = query_arguments.needs_reversed_order();

    let mut order_definitions = vec![];
    let mut joins = vec![];

    // The index is used to differentiate potentially separate relations to the same model.
    for (index, order_by) in query_arguments.order_by.iter().enumerate() {
        // This is the final column identifier to be used for the scalar field to order by.
        // - If it's on the base model with no hops, it's for example `modelTable.field`.
        // - If it is with several hops, it's the alias used for the last join, e.g.
        //   `orderby_{modelname}_{index}.field`
        let order_by_column = if !order_by.path.is_empty() {
            let last_rf = order_by.path.last().unwrap();

            Column::from((
                format!("orderby_{}_{}", &last_rf.related_model().name, index),
                order_by.field.db_name().to_owned(),
            ))
        } else {
            order_by.field.as_column()
        };

        for rf in order_by.path.iter() {
            let (left_fields, right_fields) = if rf.is_inlined_on_enclosing_model() {
                (rf.scalar_fields(), rf.referenced_fields())
            } else {
                (
                    rf.related_field().referenced_fields(),
                    rf.related_field().scalar_fields(),
                )
            };

            // `rf` is always the relation field on the left model in the join (parent).
            let left_table_alias = if rf.model().name != base_model.name {
                Some(format!("orderby_{}_{}", &rf.model().name, index))
            } else {
                None
            };

            let right_table_alias = format!("orderby_{}_{}", &rf.related_model().name, index);

            let related_model = rf.related_model();
            let pairs = left_fields.into_iter().zip(right_fields.into_iter());

            let on_conditions = pairs
                .map(|(a, b)| {
                    let a_col = if let Some(alias) = left_table_alias.clone() {
                        Column::from((alias, a.db_name().to_owned()))
                    } else {
                        a.as_column()
                    };

                    let b_col = Column::from((right_table_alias.clone(), b.db_name().to_owned()));

                    a_col.equals(b_col)
                })
                .collect::<Vec<_>>();

            joins.push(
                related_model
                    .as_table()
                    .alias(right_table_alias.clone())
                    .on(ConditionTree::single(on_conditions)),
            );
        }

        match (order_by.sort_order, needs_reversed_order) {
            (SortOrder::Ascending, true) => order_definitions.push(order_by_column.descend()),
            (SortOrder::Descending, true) => order_definitions.push(order_by_column.ascend()),
            (SortOrder::Ascending, false) => order_definitions.push(order_by_column.ascend()),
            (SortOrder::Descending, false) => order_definitions.push(order_by_column.descend()),
        }
    }

    (order_definitions, joins)
}
