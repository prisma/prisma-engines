#[macro_export]
macro_rules! assert_query {
    ($runner:expr, $q:expr, $result:expr) => {
        let result = $runner.query($q).await?;
        assert_eq!(result.to_string(), $result);
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
