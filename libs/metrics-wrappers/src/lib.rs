#[macro_export]
macro_rules! counter {
    ($name:expr, $value:expr) => {
        metrics::counter!($name, $value, "commit_id" => env!("GIT_HASH"));
    };
}

#[macro_export]
macro_rules! timing {
    ($name:expr, $value:expr) => {
        metrics::timing!($name, $value, "commit_id" => env!("GIT_HASH"));
    };

    ($name:expr, $start:expr, $end:expr) => {
        metrics::timing!($name, $start, $end, "commit_id" => env!("GIT_HASH"));
    };
}
