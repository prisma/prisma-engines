use std::fmt;

pub struct QueryForTracing<'a>(pub &'a str);

impl fmt::Display for QueryForTracing<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", strip_query_traceparent(self.0))
    }
}

pub fn strip_query_traceparent(query: &str) -> &str {
    query.rsplit_once("/* traceparent=").map_or(query, |(str, remainder)| {
        if remainder
            .split_once("*/")
            .is_some_and(|(_, suffix)| suffix.trim_end().is_empty())
        {
            str.trim_end()
        } else {
            query
        }
    })
}
