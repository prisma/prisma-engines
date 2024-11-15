use std::fmt;

/// Capitalizes first character.
/// Assumes 1-byte characters.
pub(crate) fn capitalize(s: &str) -> impl fmt::Display + '_ {
    struct Capitalized<'a>(&'a str);

    impl fmt::Display for Capitalized<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let first_char = if let Some(first_char) = self.0.chars().next() {
                first_char
            } else {
                return Ok(());
            };

            debug_assert!(first_char.is_ascii());

            let first_char = first_char.to_ascii_uppercase();
            fmt::Display::fmt(&first_char, f)?;

            f.write_str(&self.0[1..])
        }
    }

    Capitalized(s)
}

/// Compute the name of a scalar filter input.
pub(crate) fn scalar_filter_name(
    typ: &str,
    list: bool,
    nullable: bool,
    nested: bool,
    include_aggregates: bool,
) -> String {
    let list = if list { "List" } else { "" };
    let nullable = if nullable { "Nullable" } else { "" };
    let nested = if nested { "Nested" } else { "" };
    let aggregates = if include_aggregates { "WithAggregates" } else { "" };
    format!("{nested}{typ}{nullable}{list}{aggregates}Filter")
}
