#[macro_export]
macro_rules! assert_query {
    ($runner:expr, $q:expr, $result:expr) => {
        let result = $runner.query($q).await?;
        assert_eq!(result.to_string(), $result);
    };
}
