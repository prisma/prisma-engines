use super::{Sequence, SqlSchemaDescriber};
use crate::{parsers::Parser, unquote_string, ColumnType, ColumnTypeFamily, DefaultValue};
use once_cell::sync::Lazy;
use prisma_value::PrismaValue;
use regex::Regex;
use std::borrow::Cow;

pub(super) fn get_default_value(
    default_string: &str,
    data_type: &str,
    tpe: &ColumnType,
    sequences: &[Sequence],
    schema: &str,
) -> Option<DefaultValue> {
    if default_string.starts_with("NULL") {
        return None;
    }

    Some(match &tpe.family {
        ColumnTypeFamily::Int | ColumnTypeFamily::BigInt => {
            let default_expr = unsuffix_default_literal(
                &default_string,
                &[data_type, &tpe.full_data_type, "integer", "INT8", "INT4"],
            )
            .unwrap_or_else(|| default_string.into());
            let default_expr = process_string_literal(&default_expr);

            match default_expr.parse::<i64>().ok() {
                Some(int_value) => DefaultValue::value(if tpe.family.is_int() {
                    PrismaValue::Int(int_value)
                } else {
                    PrismaValue::BigInt(int_value)
                }),
                None => {
                    if let Some(seq) = is_sequence(&default_string, sequences) {
                        return Some(DefaultValue::sequence(seq));
                    }

                    if default_string.eq_ignore_ascii_case("unique_rowid()") {
                        return Some(DefaultValue::unique_rowid());
                    }

                    DefaultValue::db_generated(default_string)
                }
            }
        }
        ColumnTypeFamily::Float => match SqlSchemaDescriber::parse_float(&default_string) {
            Some(float_value) => DefaultValue::value(float_value),
            None => DefaultValue::db_generated(default_string),
        },
        ColumnTypeFamily::Decimal => match SqlSchemaDescriber::parse_float(&default_string) {
            Some(float_value) => DefaultValue::value(float_value),
            None => DefaultValue::db_generated(default_string),
        },
        ColumnTypeFamily::Boolean => match SqlSchemaDescriber::parse_bool(&default_string) {
            Some(bool_value) => DefaultValue::value(bool_value),
            None => DefaultValue::db_generated(default_string),
        },
        ColumnTypeFamily::String => match fetch_dbgenerated(&default_string) {
            Some(fun) => DefaultValue::db_generated(fun),
            None => {
                let literal = unsuffix_default_literal(&default_string, &[data_type, &tpe.full_data_type, "STRING"]);

                match literal {
                    Some(default_literal) => {
                        DefaultValue::value(process_string_literal(default_literal.as_ref()).into_owned())
                    }
                    None => DefaultValue::db_generated(default_string),
                }
            }
        },
        ColumnTypeFamily::DateTime => {
            match default_string.to_lowercase().as_str() {
                "now()"
                | "now():::timestamp"
                | "now():::timestamptz"
                | "now():::date"
                | "current_timestamp"
                | "current_timestamp():::timestamp"
                | "current_timestamp():::timestamptz"
                | "current_timestamp():::date" => DefaultValue::now(),
                _ => DefaultValue::db_generated(default_string), //todo parse values
            }
        }
        ColumnTypeFamily::Binary => DefaultValue::db_generated(default_string),
        // JSON/JSONB defaults come in the '{}'::jsonb form.
        ColumnTypeFamily::Json => unsuffix_default_literal(&default_string, &[data_type, &tpe.full_data_type])
            .map(|default| DefaultValue::value(PrismaValue::Json(unquote_string(&default))))
            .unwrap_or_else(move || DefaultValue::db_generated(default_string)),
        ColumnTypeFamily::Uuid => DefaultValue::db_generated(default_string),
        ColumnTypeFamily::Enum(enum_name) => {
            let expected_suffixes: &[Cow<'_, str>] = &[
                Cow::Borrowed(enum_name),
                Cow::Owned(format!("\"{}\"", enum_name)),
                Cow::Owned(format!("{}.{}", schema, enum_name)),
            ];
            match unsuffix_default_literal(&default_string, expected_suffixes) {
                Some(value) => DefaultValue::value(PrismaValue::Enum(SqlSchemaDescriber::unquote_string(&value))),
                None => DefaultValue::db_generated(default_string),
            }
        }
        ColumnTypeFamily::Unsupported(_) => DefaultValue::db_generated(default_string),
    })
}

fn unsuffix_default_literal<'a, T: AsRef<str>>(literal: &'a str, expected_suffixes: &[T]) -> Option<Cow<'a, str>> {
    // Tries to match expressions of the form <expr> or <expr>::<type> or <expr>:::<type>.
    static POSTGRES_DATA_TYPE_SUFFIX_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r#"(?ms)^\(?(.*?)\)?:{2,3}(\\")?(.*)(\\")?$"#).unwrap());

    let captures = POSTGRES_DATA_TYPE_SUFFIX_RE.captures(literal)?;
    let suffix = captures.get(3).unwrap().as_str();

    if !expected_suffixes
        .iter()
        .any(|expected| expected.as_ref().eq_ignore_ascii_case(suffix))
    {
        return None;
    }

    let first_capture = captures.get(1).unwrap().as_str();

    Some(Cow::Borrowed(first_capture))
}

// See https://www.postgresql.org/docs/9.3/sql-syntax-lexical.html
fn process_string_literal(literal: &str) -> Cow<'_, str> {
    // B'...' or e'...' or '...'
    static POSTGRES_STRING_DEFAULT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?ms)^(?:B|e)?'(.*)'$"#).unwrap());
    static POSTGRES_DEFAULT_QUOTE_UNESCAPE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"'(')"#).unwrap());
    static POSTGRES_DEFAULT_BACKSLASH_UNESCAPE_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r#"\\(["']|\\[^\\])"#).unwrap());
    static COCKROACH_DEFAULT_BACKSLASH_UNESCAPE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"\\\\(["']|\\)"#).unwrap());
    static POSTGRES_STRING_DEFAULTS_PIPELINE: &[(&Lazy<Regex>, &str)] = &[
        (&POSTGRES_STRING_DEFAULT_RE, "$1"),
        (&POSTGRES_DEFAULT_QUOTE_UNESCAPE_RE, "$1"),
        (&POSTGRES_DEFAULT_BACKSLASH_UNESCAPE_RE, "$1"),
        (&COCKROACH_DEFAULT_BACKSLASH_UNESCAPE_RE, "$1"),
    ];

    let mut chars = literal.chars();
    match chars.next() {
        Some('e') | Some('E') => {
            if !literal.contains('\\') {
                return Cow::Borrowed(literal);
            }

            assert!(chars.next() == Some('\''));

            let mut out = String::new();
            while let Some(char) = chars.next() {
                match char {
                    '\\' => match chars.next() {
                        Some('\\') => out.push('\\'),
                        Some('n') => out.push('\n'),
                        Some('t') => out.push('\t'),
                        Some(other) => out.push(other),
                        None => unreachable!("Backslash at end of E'' escaped string literal."),
                    },
                    '\'' => {
                        if let Some('\'') = chars.next() {
                            out.push('\'')
                        } // otherwise end of string
                    }
                    other => out.push(other),
                }
            }
            Cow::Owned(out)
        }
        _ => chain_replaces(literal, POSTGRES_STRING_DEFAULTS_PIPELINE),
    }
}
fn chain_replaces<'a>(s: &'a str, replaces: &[(&Lazy<Regex>, &str)]) -> Cow<'a, str> {
    let mut out = Cow::Borrowed(s);

    for (re, replacement) in replaces.iter() {
        if !re.is_match(out.as_ref()) {
            continue;
        }

        let replaced = re.replace_all(out.as_ref(), *replacement);

        out = Cow::Owned(replaced.into_owned())
    }

    out
}

/// Returns the name of the sequence in the schema that the defaultvalue matches if it is drawn
/// from one of them
fn is_sequence(value: &str, sequences: &[Sequence]) -> Option<String> {
    static AUTOINCREMENT_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r#"nextval\((\(?)'((.+)\.)?(("(?P<sequence>.+)")|(?P<sequence2>.+))'(::text\))?::(regclass|REGCLASS)\)"#,
        )
        .expect("compile autoincrement regex")
    });

    AUTOINCREMENT_REGEX.captures(value).and_then(|captures| {
        let sequence_name = captures.name("sequence").or_else(|| captures.name("sequence2"));

        sequence_name.and_then(|name| {
            sequences
                .iter()
                .find(|seq| seq.name == name.as_str())
                .map(|x| x.name.clone())
        })
    })
}

fn fetch_dbgenerated(value: &str) -> Option<String> {
    static POSTGRES_DB_GENERATED_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r#"(^\((.*)\)):{2,3}(\\")?(.*)(\\")?$"#).unwrap());

    let captures = POSTGRES_DB_GENERATED_RE.captures(value)?;
    let fun = captures.get(1).unwrap().as_str();
    let suffix = captures.get(4).unwrap().as_str();
    Some(format!("{}::{}", fun, suffix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres_is_sequence_works() {
        let sequences = vec![
            Sequence {
                name: "first_sequence".to_string(),
                ..Default::default()
            },
            Sequence {
                name: "second_sequence".to_string(),
                ..Default::default()
            },
            Sequence {
                name: "third_Sequence".to_string(),
                ..Default::default()
            },
            Sequence {
                name: "fourth_Sequence".to_string(),
                ..Default::default()
            },
            Sequence {
                name: "fifth_sequence".to_string(),
                ..Default::default()
            },
        ];

        let first_autoincrement = r#"nextval('first_sequence'::regclass)"#;
        assert!(is_sequence(first_autoincrement, &sequences).is_some());

        let second_autoincrement = r#"nextval('schema_name.second_sequence'::regclass)"#;
        assert!(is_sequence(second_autoincrement, &sequences).is_some());

        let third_autoincrement = r#"nextval('"third_Sequence"'::regclass)"#;
        assert!(is_sequence(third_autoincrement, &sequences).is_some());

        let fourth_autoincrement = r#"nextval('"schema_Name"."fourth_Sequence"'::regclass)"#;
        assert!(is_sequence(fourth_autoincrement, &sequences).is_some());

        let fifth_autoincrement = r#"nextval(('fifth_sequence'::text)::regclass)"#;
        assert!(is_sequence(fifth_autoincrement, &sequences).is_some());

        let non_autoincrement = r#"string_default_named_seq"#;
        assert!(is_sequence(non_autoincrement, &sequences).is_none());
    }
}
