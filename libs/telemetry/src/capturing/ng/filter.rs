use std::{borrow::Cow, sync::LazyLock};

use tracing::Metadata;
use tracing_subscriber::{filter::filter_fn, layer::Filter, EnvFilter};

pub static SHOW_ALL_TRACES: LazyLock<bool> = LazyLock::new(|| {
    std::env::var("PRISMA_SHOW_ALL_TRACES")
        .map(|enabled| enabled.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
});

fn is_user_facing_span(meta: &Metadata<'_>) -> bool {
    if *SHOW_ALL_TRACES {
        return true;
    }
    meta.is_span() && meta.fields().iter().any(|f| f.name() == "user_facing")
}

pub fn user_facing_spans_and_events<S>() -> impl Filter<S> {
    filter_fn(|meta| is_user_facing_span(meta) || meta.is_event())
}

pub fn user_facing_spans<S>() -> impl Filter<S> {
    filter_fn(|meta| is_user_facing_span(meta))
}

pub enum QueryEngineLogLevel<'a> {
    FromEnv,
    Override(&'a str),
}

impl<'a> QueryEngineLogLevel<'a> {
    fn level(self) -> Option<Cow<'a, str>> {
        match self {
            Self::FromEnv => std::env::var("QE_LOG_LEVEL").ok().map(<_>::into),
            Self::Override(level) => Some(level.into()),
        }
    }
}

pub struct EnvFilterBuilder<'a> {
    log_queries: bool,
    log_level: QueryEngineLogLevel<'a>,
}

impl<'a> EnvFilterBuilder<'a> {
    pub fn new() -> Self {
        Self {
            log_queries: false,
            log_level: QueryEngineLogLevel::FromEnv,
        }
    }

    pub fn log_queries(mut self, log_queries: bool) -> Self {
        self.log_queries = log_queries;
        self
    }

    pub fn with_log_level(mut self, level: &'a str) -> Self {
        self.log_level = QueryEngineLogLevel::Override(level);
        self
    }

    pub fn build(self) -> EnvFilter {
        let level = self.log_level.level().unwrap_or("error".into());

        let mut filter = EnvFilter::from_default_env()
            .add_directive("h2=error".parse().unwrap())
            .add_directive("hyper=error".parse().unwrap())
            .add_directive("tower=error".parse().unwrap())
            .add_directive(format!("query_engine={level}").parse().unwrap())
            .add_directive(format!("query_core={level}").parse().unwrap())
            .add_directive(format!("query_connector={level}").parse().unwrap())
            .add_directive(format!("sql_query_connector={level}").parse().unwrap())
            .add_directive(format!("mongodb_query_connector={level}").parse().unwrap());

        if self.log_queries {
            filter = filter
                .add_directive("quaint[{is_query}]=trace".parse().unwrap())
                .add_directive("mongodb_query_connector[{is_query}]=debug".parse().unwrap());
        }

        filter
    }
}
