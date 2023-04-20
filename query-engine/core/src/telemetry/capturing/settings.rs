use std::collections::HashSet;

static VALID_VALUES: &[&str] = &["error", "warn", "info", "debug", "trace", "query", "tracing"];

#[derive(Debug, Clone, Default)]
pub struct Settings {
    /// only capture log events from the specified log levels, the special level "query", which does not
    /// exist in the engine logging infrastructure, is shimed from any event describing a query, regardless
    /// of its level.
    pub(super) included_log_levels: HashSet<String>,
    /// whether to include trace spans when capturing
    pub(super) include_traces: bool,
}

impl Settings {
    pub(super) fn new(included_log_levels: HashSet<String>, include_traces: bool) -> Self {
        Self {
            include_traces,
            included_log_levels,
        }
    }
}

/// As the test below shows, settings can be constructed from a comma separated string.
/// Examples: valid: `"error, warn, query, tracing"` invalid: "foo, bar baz". strings corresponding
/// passed in are trimmed and converted to lowercase. chunks corresponding to levels different from
/// those in VALID_LEVELS are ignored.
///
/// The ttl is always the same (DEFAULT_TTL) but is there to allow for easier unit-testing of c
/// apturing logic
impl From<&str> for Settings {
    fn from(s: &str) -> Self {
        let chunks = s.split(',');
        let mut set = HashSet::from_iter(
            chunks
                .into_iter()
                .map(str::trim)
                .map(str::to_lowercase)
                .filter(|s| VALID_VALUES.contains(&s.as_str())),
        );

        let include_traces = set.remove("tracing");
        let included_log_levels = set;

        Self::new(included_log_levels, include_traces)
    }
}

impl Settings {
    pub fn is_enabled(&self) -> bool {
        self.traces_enabled() || self.logs_enabled()
    }

    pub fn traces_enabled(&self) -> bool {
        self.include_traces
    }

    pub fn logs_enabled(&self) -> bool {
        !self.included_log_levels.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_options_from() {
        let options = Settings::from("error, warn, query, tracing");
        assert_eq!(options.included_log_levels.len(), 3);
        assert!(options.included_log_levels.contains("error"));
        assert!(options.included_log_levels.contains("warn"));
        assert!(options.included_log_levels.contains("query"));
        assert!(options.include_traces);
        assert!(options.is_enabled());

        let options = Settings::from("foo, bar baz");
        assert!(!options.is_enabled());
    }
}
