#[macro_export]
macro_rules! assert_query {
    ($runner:expr, $q:expr, $result:expr) => {
        let result = $runner.query($q).await?;
        assert_eq!(result.to_string(), $result);
    };
}

#[macro_export]
macro_rules! match_connector_result {
    ($runner:expr, $q:expr, $( $($matcher:pat_param)|+ $( if $pred:expr )? => $result:expr ),*) => {
        use query_tests_setup::*;
        use query_tests_setup::ConnectorVersion::*;

        let connector = $runner.connector_version();

        let mut results = match &connector {
            $(
                $( $matcher )|+ $( if $pred )? => $result
            ),*
        };

        let query_result = $runner.query($q).await?.to_string();

        if results.len() == 0 {
            panic!("No results defined for connector {connector}. Query result: {query_result}");
        }

        assert_eq!(
            results.contains(&query_result.as_str()),
            true,
            "Query result: {query_result} is not part of the expected results: {results:?} for connector {connector}",
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
    ($runner:expr, $q:expr) => {{
        let res = $runner.query($q.to_string()).await?;
        res.assert_success();
        res.to_string()
    }};
}

#[macro_export]
macro_rules! run_query_pretty {
    ($runner:expr, $q:expr) => {{
        let res = $runner.query($q.to_string()).await?;
        res.assert_success();
        res.to_string_pretty()
    }};
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

#[macro_export]
macro_rules! retry {
    ($body:block, $times:expr) => {{
        use std::time::Duration;
        use tokio::time::sleep;

        let mut retries = $times;

        loop {
            let res = $body.await?;

            if !res.failed() {
                break res;
            }

            if retries > 0 {
                retries -= 1;
                sleep(Duration::from_millis(5)).await;
                continue;
            }

            break res;
        }
    }};
}

#[macro_export]
macro_rules! with_id_excess {
    ($runner:expr, $query_template:expr) => {{
        let max_bind_values = $runner
            .max_bind_values()
            .expect("Test expected to run only for relational databases.");

        let cycle = |argn: usize| (argn % 10 + 1).to_string();
        let id_list = (0..=max_bind_values).map(cycle).collect::<Vec<_>>().join(",");
        $query_template.replace(":id_list:", &id_list)
    }};
}
