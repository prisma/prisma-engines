use query_tests_setup::Runner;

mod batch;
mod queries;

fn used_db_join_times(logs: &[String]) -> usize {
    logs.iter()
        .filter(|l| l.contains("LEFT JOIN LATERAL") || (l.contains("JSON_ARRAYAGG") && l.contains("JSON_OBJECT")))
        .count()
}

async fn assert_used_lateral_join(runner: &mut Runner, expected: bool) {
    let logs = runner.get_logs().await;
    let actual = used_db_join_times(&logs) != 0;

    assert_eq!(
        actual, expected,
        "expected lateral join to be used: {expected}, instead it was: {actual}"
    );
}
