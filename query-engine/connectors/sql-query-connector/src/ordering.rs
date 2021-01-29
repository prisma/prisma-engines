use crate::query_arguments_ext::QueryArgumentsExt;
use connector_interface::QueryArguments;
use datamodel::ScalarField;
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

    for order_by in query_arguments.order_by.iter() {
        let target_model = order_by
            .path
            .last()
            .map(|rf| rf.related_model())
            .unwrap_or(base_model.clone());

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
                Some(format!("orderby_{}", &rf.model().name))
            } else {
                None
            };

            let right_table_alias = format!("orderby_{}", &rf.related_model().name);

            let related_model = rf.related_model();
            let pairs = left_fields
                .into_iter()
                .zip(right_fields.into_iter())
                .collect::<Vec<_>>();

            let on_conditions = pairs
                .into_iter()
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

        let col = if joins.len() > 0 {
            Column::from((
                format!("orderby_{}", &target_model.name),
                order_by.field.db_name().to_owned(),
            ))
        } else {
            order_by.field.as_column()
        };

        match (order_by.sort_order, needs_reversed_order) {
            (SortOrder::Ascending, true) => order_definitions.push(col.descend()),
            (SortOrder::Descending, true) => order_definitions.push(col.ascend()),
            (SortOrder::Ascending, false) => order_definitions.push(col.ascend()),
            (SortOrder::Descending, false) => order_definitions.push(col.descend()),
        }
    }

    (order_definitions, joins)
}
