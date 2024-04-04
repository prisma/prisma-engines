use query_tests_setup::{
    query_core::{BatchDocument, QueryDocument},
    GraphqlBody, MultiQuery, Runner, TestResult,
};

use crate::run_query;

pub async fn compact_batch(runner: &Runner, queries: Vec<String>) -> TestResult<BatchDocument> {
    // Ensure individual queries are valid. Helps to debug tests when writing them.
    for q in queries.iter() {
        run_query!(runner, q.to_string());
    }

    // Ensure batched queries are valid
    runner.batch(queries.clone(), false, None).await?.assert_success();

    let doc = GraphqlBody::Multi(MultiQuery::new(
        queries.into_iter().map(Into::into).collect(),
        false,
        None,
    ))
    .into_doc()
    .unwrap();
    let batch = match doc {
        QueryDocument::Multi(batch) => batch.compact(runner.query_schema()),
        _ => unreachable!(),
    };

    Ok(batch.compact(runner.query_schema()))
}
