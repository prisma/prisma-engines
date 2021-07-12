#[macro_export]
macro_rules! assert_query {
    ($runner:expr, $q:expr, $result:expr) => {
        let result = $runner.query($q).await?;
        assert_eq!(result.to_string(), $result);
    };
}

#[macro_export]
macro_rules! assert_query_many {
    ($runner:expr, $q:expr, $potential_results:expr) => {
        let query_result = $runner.query($q).await?.to_string();

        assert_eq!(
            $potential_results.contains(&query_result.as_str()),
            true,
            "Query result: {} is not part of the expected results: {:?}",
            query_result,
            $potential_results
        );
    };
}

#[macro_export]
macro_rules! is_one_of {
    ($result:expr, $potential_results:expr) => {
        assert_eq!(
            $potential_results.contains(&$result.as_str()),
            true,
            "Query result: {} is not part of the expected results: {:?}",
            $result,
            $potential_results
        );
    };
}

#[macro_export]
macro_rules! run_query {
    ($runner:expr, $q:expr) => {
        $runner.query($q).await?.to_string()
    };
}

#[macro_export]
macro_rules! run_query_json {
    ($runner:expr, $q:expr) => {
        serde_json::from_str::<serde_json::Value>($runner.query($q).await?.to_string().as_str()).unwrap()
    };
    ($runner:expr, $q:expr, $path: expr) => {
        query_tests_setup::walk_json(
            &serde_json::from_str::<serde_json::Value>($runner.query($q).await?.to_string().as_str()).unwrap(),
            $path,
        )
        .unwrap()
        .to_owned()
    };
}

#[macro_export]
macro_rules! assert_error {
    ($runner:expr, $q:expr, $code:expr) => {
        $runner.query($q).await?.assert_failure($code, None);
    };
    ($runner:expr, $q:expr, $code:expr, $msg:expr) => {
        $runner.query($q).await?.assert_failure($code, Some($msg.to_string()));
    };
}
