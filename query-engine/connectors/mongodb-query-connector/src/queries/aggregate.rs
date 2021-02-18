use connector_interface::*;
use mongodb::Database;
use prisma_models::prelude::*;

pub async fn aggregate(
    database: &Database,
    model: &ModelRef,
    query_arguments: QueryArguments,
    selections: Vec<AggregationSelection>,
    group_by: Vec<ScalarFieldRef>,
    having: Option<Filter>,
) -> crate::Result<Vec<AggregationRow>> {
    let coll = database.collection(&model.db_name());

    // if !group_by.is_empty() {
    //     group_by_aggregate(conn, model, query_arguments, selections, group_by, having).await
    // } else {
    //     plain_aggregate(conn, model, query_arguments, selections)
    //         .await
    //         .map(|v| vec![v])
    // }

    todo!()
}
