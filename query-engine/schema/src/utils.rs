pub fn capitalize<T>(s: T) -> String
where
    T: Into<String>,
{
    let s = s.into();

    // This is safe to unwrap, as the validation regex for model / field
    // names used in the data model essentially guarantees ASCII.
    let first_char = s.chars().next().unwrap();

    format!("{}{}", first_char.to_uppercase(), s[1..].to_owned())
}

/// Compute the name of a scalar filter input.
pub fn scalar_filter_name(typ: &str, list: bool, nullable: bool, nested: bool, include_aggregates: bool) -> String {
    let list = if list { "List" } else { "" };
    let nullable = if nullable { "Nullable" } else { "" };
    let nested = if nested { "Nested" } else { "" };
    let aggregates = if include_aggregates { "WithAggregates" } else { "" };
    format!("{nested}{typ}{nullable}{list}{aggregates}Filter")
}
