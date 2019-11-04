#[macro_export]
macro_rules! counter {
    ($name:expr, $value:expr) => {
        match std::env::var("TEST_CONTEXT") {
            Ok(context) => {
                metrics::counter!($name, $value, "commit_id" => env!("GIT_HASH"), "context" => context);
            },
            _ => {
                metrics::counter!($name, $value, "commit_id" => env!("GIT_HASH"));
            },
        }
    };
}

#[macro_export]
macro_rules! timing {
    ($name:expr, $value:expr) => {
        match std::env::var("TEST_CONTEXT") {
            Ok(context) => {
                metrics::timing!($name, $value, "commit_id" => env!("GIT_HASH"), "context" => context);
            },
            _ => {
                metrics::timing!($name, $value, "commit_id" => env!("GIT_HASH"));
            },
        }
    };

    ($name:expr, $start:expr, $end:expr) => {
        match std::env::var("TEST_CONTEXT") {
            Ok(context) => {
                metrics::timing!($name, $start, $end, "commit_id" => env!("GIT_HASH"), "context" => context);
            },
            _ => {
                metrics::timing!($name, $start, $end, "commit_id" => env!("GIT_HASH"));
            },
        }
    };
}
