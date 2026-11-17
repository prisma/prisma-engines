//! Oracle literals and the documented KES decompiler forms. Unknown expressions
//! remain dbgenerated; their original spelling is never rewritten or executed.
use crate::{ColumnTypeFamily, DefaultValue, PrismaValue};
use psl::builtin_connectors::KingbaseOracleType;
use regex::Regex;
use std::{str::FromStr, sync::LazyLock};

/// Compare numeric literal cast chains without changing or evaluating their SQL.
/// Only exact catalog decorations can be dropped; rounding/overflow casts stay.
/// Callers must restrict this comparison to numeric columns.
pub fn numeric_default_expressions_match(previous: &str, next: &str) -> bool {
    use sqlparser::{
        ast::{CastKind, DataType, ExactNumberInfo, Expr, UnaryOperator, Value},
        dialect::PostgreSqlDialect,
        parser::Parser,
        tokenizer::Token,
    };

    #[derive(PartialEq)]
    struct NumericExpression {
        literal: bigdecimal::BigDecimal,
        casts: Vec<(u64, i64)>,
        quoted: bool,
    }

    fn normalize(expression: &Expr) -> Option<NumericExpression> {
        match expression {
            Expr::Nested(inner) => normalize(inner),
            Expr::Value(value) => {
                let (literal, quoted) = match &value.value {
                    Value::Number(number, false) => (number, false),
                    Value::SingleQuotedString(number) => (number, true),
                    _ => return None,
                };
                Some(NumericExpression {
                    literal: literal.parse().ok()?,
                    casts: Vec::new(),
                    quoted,
                })
            }
            Expr::UnaryOp {
                op: op @ (UnaryOperator::Minus | UnaryOperator::Plus),
                expr,
            } => {
                let mut inner = normalize(expr)?;
                if inner.quoted || !inner.casts.is_empty() {
                    return None;
                }
                if *op == UnaryOperator::Minus {
                    inner.literal = -inner.literal;
                }
                Some(inner)
            }
            Expr::Cast {
                kind: kind @ (CastKind::DoubleColon | CastKind::Cast),
                expr,
                data_type,
                array: false,
                format: None,
            } => {
                let info = match data_type {
                    DataType::Numeric(info) | DataType::Decimal(info) => *info,
                    DataType::Custom(name, args) if name.to_string().eq_ignore_ascii_case("NUMBER") => {
                        match args.as_slice() {
                            [p] => ExactNumberInfo::Precision(p.parse().ok()?),
                            [p, s] => ExactNumberInfo::PrecisionAndScale(p.parse().ok()?, s.parse().ok()?),
                            _ => return None,
                        }
                    }
                    _ => return None,
                };
                let mut inner = normalize(expr)?;
                inner.quoted = false;
                let (precision, scale) = match info {
                    // Do not assume Oracle's parameterless CAST is unbounded.
                    ExactNumberInfo::None if matches!(kind, CastKind::Cast) => return None,
                    ExactNumberInfo::None => return Some(inner),
                    ExactNumberInfo::Precision(p) => (p, 0),
                    ExactNumberInfo::PrecisionAndScale(p, s) => (p, s),
                };
                if !(1..=1000).contains(&precision) || !(0..=1000).contains(&scale) {
                    return None;
                }
                // R2 raises numeric precision to scale in both CAST and the
                // catalog. Compare the physical cast without evaluating it.
                let precision = precision.max(scale as u64);
                let (coefficient, fractional_digits) = inner.literal.normalized().as_bigint_and_exponent();
                let coefficient = coefficient.to_string();
                let integer_digits =
                    (coefficient.trim_start_matches('-').len() as i64).checked_sub(fractional_digits)?;
                let fits =
                    coefficient == "0" || fractional_digits <= scale && integer_digits <= precision as i64 - scale;
                // Once a cast can alter the literal, later casts cannot be
                // judged against the original literal. Preserve that chain.
                if !inner.casts.is_empty() || !fits {
                    inner.casts.push((precision, scale));
                }
                Some(inner)
            }
            _ => None,
        }
    }

    let parse = |input: &str| {
        // R2's catalog uses ::numeric even for Oracle CAST(... AS NUMBER).
        let mut parser = Parser::new(&PostgreSqlDialect {}).try_with_sql(input).ok()?;
        let expression = parser.parse_expr().ok()?;
        if parser.peek_token().token != Token::EOF {
            return None;
        }
        let normalized = normalize(&expression)?;
        (!normalized.quoted).then_some(normalized)
    };
    match (parse(previous), parse(next)) {
        (Some(previous), Some(next)) => previous == next,
        _ => false,
    }
}

#[cfg(test)]
pub(super) fn parse(input: &str, family: &ColumnTypeFamily) -> Option<DefaultValue> {
    parse_inner(input, family, None, None, false, None)
}

pub(super) fn parse_native(
    input: &str,
    family: &ColumnTypeFamily,
    native: Option<&KingbaseOracleType>,
) -> Option<DefaultValue> {
    // DEFAULT NULL keeps a typed NULL in R2's catalog. Only recognize known
    // physical casts here; non-NULL values still use the stricter rules below.
    let null_family = if matches!(native, Some(KingbaseOracleType::Number(_))) {
        &ColumnTypeFamily::Decimal
    } else {
        family
    };
    let null_casts: &[&str] = match native {
        Some(KingbaseOracleType::Date | KingbaseOracleType::Timestamp(_) | KingbaseOracleType::TimestampLocalTz(_)) => {
            &["timestamp", "timestamp without time zone"]
        }
        Some(KingbaseOracleType::TimestampTz(_)) => &["timestamptz", "timestamp with time zone"],
        Some(KingbaseOracleType::Blob) => &["bytea"],
        _ => &[],
    };
    if strip_casts(input.trim(), |tail| {
        harmless_casts(tail, null_family, None, None, false, None, null_casts)
    })
    .eq_ignore_ascii_case("null")
    {
        return None;
    }
    let floating_cast = match native {
        Some(KingbaseOracleType::BinaryFloat | KingbaseOracleType::Float(Some(1..=24))) => Some("real"),
        Some(KingbaseOracleType::BinaryDouble | KingbaseOracleType::Float(None | Some(25..=53))) => {
            Some("double precision")
        }
        _ => None,
    };
    let fixed_character = matches!(
        native,
        Some(KingbaseOracleType::Char(_) | KingbaseOracleType::NChar(_) | KingbaseOracleType::CharByte(_))
    );
    let native_string_cast = match native {
        Some(KingbaseOracleType::Uuid) => Some("uuid"),
        Some(KingbaseOracleType::Xml) => Some("xml"),
        Some(KingbaseOracleType::CharByte(_)) => Some("bpcharbyte"),
        Some(KingbaseOracleType::VarChar2Byte(_)) => Some("varcharbyte"),
        Some(KingbaseOracleType::RowId) => Some("rowid"),
        Some(KingbaseOracleType::URowId(_)) => Some("urowid"),
        _ => None,
    };
    parse_inner(input, family, None, floating_cast, fixed_character, native_string_cast)
}

pub(super) fn parse_enum(input: &str, family: &ColumnTypeFamily, namespace: &str, name: &str) -> Option<DefaultValue> {
    parse_inner(input, family, Some((namespace, name)), None, false, None)
}

fn parse_inner(
    input: &str,
    family: &ColumnTypeFamily,
    enm: Option<(&str, &str)>,
    floating_cast: Option<&str>,
    fixed_character: bool,
    native_string_cast: Option<&str>,
) -> Option<DefaultValue> {
    let allowed = |tail: &str| {
        harmless_casts(
            tail,
            family,
            enm,
            floating_cast,
            fixed_character,
            native_string_cast,
            &[],
        )
    };
    let expression = strip_casts(input.trim(), allowed);
    if expression.eq_ignore_ascii_case("null") {
        return None;
    }
    if let Some(sequence) = sequence(expression)
        && expression == uncast(expression)
    {
        return Some(DefaultValue::sequence(sequence));
    }
    if matches!(family, ColumnTypeFamily::DateTime)
        && ["current_timestamp", "current_timestamp()", "now()"]
            .iter()
            .any(|v| expression.eq_ignore_ascii_case(v))
    {
        return Some(DefaultValue::now());
    }
    if matches!(family, ColumnTypeFamily::Binary)
        && let Some(bytes) = binary(expression)
    {
        return Some(DefaultValue::value(PrismaValue::Bytes(bytes)));
    }
    let literal = quoted(expression)
        .filter(|(_, tail)| allowed(tail))
        .map(|(value, _)| value);
    let plain = expression;
    let scalar = literal.as_deref().unwrap_or(plain);
    let value = match family {
        ColumnTypeFamily::Int => scalar.parse::<i64>().ok().map(PrismaValue::Int),
        ColumnTypeFamily::BigInt => scalar.parse::<i64>().ok().map(PrismaValue::BigInt),
        ColumnTypeFamily::Decimal | ColumnTypeFamily::Float => {
            bigdecimal::BigDecimal::from_str(scalar).ok().map(PrismaValue::Float)
        }
        ColumnTypeFamily::Boolean => match scalar.to_ascii_lowercase().as_str() {
            "true" | "1" => Some(PrismaValue::Boolean(true)),
            "false" | "0" => Some(PrismaValue::Boolean(false)),
            _ => None,
        },
        ColumnTypeFamily::String => literal.map(PrismaValue::String),
        ColumnTypeFamily::Json => literal.map(PrismaValue::Json),
        ColumnTypeFamily::Enum(_) => literal.map(PrismaValue::Enum),
        _ => None,
    };
    Some(value.map_or_else(|| DefaultValue::db_generated(input), DefaultValue::value))
}

fn valid_casts(tail: &str) -> bool {
    let tail = tail.trim();
    if tail.is_empty() {
        return true;
    }
    static CASTS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^(?:::\s*(?:[A-Za-z_][A-Za-z_0-9]*|"(?:[^"]|"")+")(?:\.[A-Za-z_][A-Za-z_0-9]*)?(?:\s+(?:varying|precision|with|without|time|zone))*(?:\s*\(\s*\d+(?:\s*,\s*\d+)?\s*\))?(?:\[\])?\s*)+$"#).unwrap()
    });
    CASTS.is_match(tail)
}

fn uncast(input: &str) -> &str {
    strip_casts(input, valid_casts)
}

fn harmless_casts(
    tail: &str,
    family: &ColumnTypeFamily,
    enm: Option<(&str, &str)>,
    floating_cast: Option<&str>,
    fixed_character: bool,
    native_string_cast: Option<&str>,
    extra_builtin_casts: &[&str],
) -> bool {
    let quote = |name: &str| format!("\"{}\"", name.replace('"', "\"\""));
    let mut names = match family {
        ColumnTypeFamily::String => {
            let mut casts = vec!["text", "varchar", "character varying"];
            // These catalog casts only belong to their matching native column.
            // Other string columns must preserve UUID/XML conversion expressions.
            casts.extend(native_string_cast);
            if fixed_character {
                casts.extend(["bpchar", "character"]);
            }
            casts
        }
        ColumnTypeFamily::Json => vec!["json", "jsonb", "text"],
        ColumnTypeFamily::Boolean => vec!["bool", "boolean"],
        ColumnTypeFamily::Int | ColumnTypeFamily::BigInt => {
            vec![
                "numeric", "decimal", "integer", "int4", "bigint", "int8", "smallint", "int2",
            ]
        }
        ColumnTypeFamily::Decimal => vec!["numeric", "decimal"],
        ColumnTypeFamily::Float => {
            let mut casts = vec!["numeric", "decimal"];
            casts.extend(floating_cast);
            casts
        }
        ColumnTypeFamily::Enum(_) => vec!["text"],
        _ => vec![],
    };
    names.extend(extra_builtin_casts.iter().copied());
    let mut names: Vec<String> = names
        .into_iter()
        .flat_map(|name| {
            [
                name.to_owned(),
                format!("sys_catalog.{name}"),
                format!("pg_catalog.{name}"),
            ]
        })
        .collect();
    if let Some(name @ ("bpcharbyte" | "varcharbyte" | "rowid" | "urowid")) = native_string_cast {
        names.extend([
            quote(name),
            format!("sys.{name}"),
            format!("sys.{}", quote(name)),
            format!("pg_catalog.{}", quote(name)),
        ]);
    }
    if let Some((namespace, name)) = enm {
        names.push(quote(name));
        names.push(format!("{}.{}", quote(namespace), quote(name)));
        names.push(format!("{}.{}", namespace, quote(name)));
        if name == name.to_ascii_lowercase() {
            names.push(name.to_owned());
            names.push(format!("{namespace}.{name}"));
        }
    }
    names.sort_by_key(|name| std::cmp::Reverse(name.len()));
    let mut tail = tail.trim();
    let mut fixed_cast = false;
    while !tail.is_empty() {
        let Some(rest) = tail.strip_prefix("::") else {
            return false;
        };
        let rest = rest.trim_start();
        let Some((name, end)) = names.iter().find_map(|name| {
            rest.strip_prefix(name)
                .filter(|after| after.trim().is_empty() || after.trim_start().starts_with("::"))
                .map(|after| (name, after))
        }) else {
            return false;
        };
        let is_fixed = matches!(
            name.rsplit('.').next().map(|name| name.trim_matches('"')),
            Some("bpchar" | "character" | "bpcharbyte")
        );
        // A later bpchar-to-text/varchar cast removes trailing spaces even
        // when the destination column is CHAR. Keep that whole expression.
        if fixed_cast && !is_fixed {
            return false;
        }
        fixed_cast |= is_fixed;
        tail = end.trim();
    }
    true
}

fn strip_casts(mut input: &str, allowed: impl Fn(&str) -> bool) -> &str {
    loop {
        input = unparen(input.trim());
        let mut depth = 0;
        let mut offset = 0;
        let mut end = None;
        while offset < input.len() {
            let tail = &input[offset..];
            if let Some((_, rest)) = quoted(tail) {
                offset = input.len() - rest.len();
                continue;
            }
            if depth == 0 && tail.starts_with("::") {
                // An allowed outer cast cannot make an earlier semantic cast
                // disappear (for example bpchar::text trims trailing spaces).
                if !allowed(tail) {
                    return input;
                }
                end = Some(offset);
                break;
            }
            let ch = tail.chars().next().unwrap();
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
            }
            offset += ch.len_utf8();
        }
        match end {
            Some(end) => input = &input[..end],
            None => return input,
        }
    }
}

fn unparen(mut input: &str) -> &str {
    loop {
        if !input.starts_with('(') {
            return input;
        }
        let mut depth = 0;
        let mut offset = 0;
        let mut end = None;
        while offset < input.len() {
            let tail = &input[offset..];
            if let Some((_, rest)) = quoted(tail) {
                offset = input.len() - rest.len();
                continue;
            }
            let ch = tail.chars().next().unwrap();
            if ch == '(' {
                depth += 1;
            }
            if ch == ')' {
                depth -= 1;
                if depth == 0 {
                    end = Some(offset);
                    break;
                }
            }
            offset += ch.len_utf8();
        }
        if end == Some(input.len() - 1) {
            input = input[1..input.len() - 1].trim();
        } else {
            return input;
        }
    }
}

fn quoted(input: &str) -> Option<(String, &str)> {
    let bytes = input.as_bytes();
    let prefix = if matches!(bytes.first(), Some(b'n' | b'N' | b'e' | b'E')) {
        1
    } else {
        0
    };
    let q = if matches!(bytes.first(), Some(b'n' | b'N')) {
        1
    } else {
        0
    };
    if matches!(bytes.get(q), Some(b'q' | b'Q')) && bytes.get(q + 1) == Some(&b'\'') {
        let open = input[q + 2..].chars().next()?;
        let close = match open {
            '[' => ']',
            '{' => '}',
            '(' => ')',
            '<' => '>',
            other => other,
        };
        let start = q + 2 + open.len_utf8();
        let marker = format!("{close}'");
        let length = input[start..].find(&marker)?;
        return Some((
            input[start..start + length].to_owned(),
            &input[start + length + marker.len()..],
        ));
    }
    if bytes.get(prefix) != Some(&b'\'') {
        return None;
    }
    let escapes = matches!(bytes.first(), Some(b'e' | b'E'));
    let mut value = String::new();
    let mut chars = input[prefix + 1..].char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        if ch == '\'' {
            if chars.peek().is_some_and(|(_, next)| *next == '\'') {
                chars.next();
                value.push('\'');
            } else {
                return Some((value, &input[prefix + 1 + index + 1..]));
            }
        } else if escapes && ch == '\\' {
            let (_, next) = chars.next()?;
            value.push(match next {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '\\' => '\\',
                '\'' => '\'',
                _ => return None,
            });
        } else {
            value.push(ch);
        }
    }
    None
}

pub(super) fn sequence(input: &str) -> Option<String> {
    let expression = uncast(input.trim());
    let lower = expression.to_ascii_lowercase();
    let name = if lower.ends_with(".nextval") {
        &expression[..expression.len() - 8]
    } else {
        let open = expression.find('(')?;
        if !["nextval", "sys_catalog.nextval", "pg_catalog.nextval"].contains(&lower[..open].trim())
            || !expression.ends_with(')')
        {
            return None;
        }
        let (value, tail) = quoted(uncast(expression[open + 1..expression.len() - 1].trim()))?;
        if !valid_casts(tail) {
            return None;
        }
        return identifier(&value);
    };
    identifier(name)
}

fn identifier(input: &str) -> Option<String> {
    static IDENT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^(?:[A-Za-z_][A-Za-z_0-9$]*|"(?:[^"]|"")+")(?:\.(?:[A-Za-z_][A-Za-z_0-9$]*|"(?:[^"]|"")+"))*$"#)
            .unwrap()
    });
    if !IDENT.is_match(input.trim()) {
        return None;
    }
    // Keep qualification and quoting intact for nontrivial external sequences.
    let input = input.trim();
    if !input.contains('.')
        && input.starts_with('"')
        && !input.chars().any(|ch| ch.is_uppercase())
        && !input[1..input.len() - 1].contains("\"\"")
    {
        Some(input[1..input.len() - 1].replace("\"\"", "\""))
    } else {
        Some(input.to_owned())
    }
}

fn binary(input: &str) -> Option<Vec<u8>> {
    let open = input.find('(')?;
    let function = input[..open].trim();
    if !input.ends_with(')') {
        return None;
    }
    let (hex, rest) = quoted(input[open + 1..input.len() - 1].trim())?;
    let allowed = if function.eq_ignore_ascii_case("hextoraw") {
        rest.trim().is_empty()
    } else if function.eq_ignore_ascii_case("decoding") || function.eq_ignore_ascii_case("decode") {
        let comma = rest.find(',')?;
        valid_casts(&rest[..comma])
            && quoted(rest[comma + 1..].trim())
                .is_some_and(|(format, tail)| format.eq_ignore_ascii_case("hex") && valid_casts(tail))
    } else {
        false
    };
    if !allowed || hex.len() % 2 != 0 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DefaultKind;

    #[test]
    fn native_time_and_blob_null_defaults_preserve_non_null_expressions() {
        for (native, family, cast) in [
            (
                KingbaseOracleType::Date,
                ColumnTypeFamily::DateTime,
                "timestamp without time zone",
            ),
            (
                KingbaseOracleType::Timestamp(Some(3)),
                ColumnTypeFamily::DateTime,
                "timestamp without time zone",
            ),
            (
                KingbaseOracleType::TimestampTz(Some(3)),
                ColumnTypeFamily::DateTime,
                "timestamp with time zone",
            ),
            (
                KingbaseOracleType::TimestampLocalTz(Some(3)),
                ColumnTypeFamily::DateTime,
                "timestamp without time zone",
            ),
            (KingbaseOracleType::Blob, ColumnTypeFamily::Binary, "bytea"),
        ] {
            for expression in [
                format!("NULL::{cast}"),
                format!("((NULL)::sys_catalog.{cast})"),
                format!("(NULL::pg_catalog.{cast})::{cast}"),
            ] {
                assert!(
                    parse_native(&expression, &family, Some(&native)).is_none(),
                    "{native:?}: {expression}"
                );
                assert!(matches!(
                    parse_native(&expression, &family, None).unwrap().kind(),
                    DefaultKind::DbGenerated(_)
                ));
            }
            for expression in [
                format!("'NULL'::{cast}"),
                format!("NULLIF(1,2)::{cast}"),
                format!("NULL::custom_domain::{cast}"),
                format!("NULL::{cast}::custom_domain"),
                format!("NULL::{cast} + 1"),
            ] {
                assert_eq!(
                    parse_native(&expression, &family, Some(&native)).unwrap().kind(),
                    &DefaultKind::DbGenerated(Some(expression.clone())),
                    "{native:?}: {expression}"
                );
            }
        }
    }

    #[test]
    fn unsupported_number_null_defaults_keep_expressions_distinct() {
        let native = KingbaseOracleType::Number(
            psl::builtin_connectors::KingbaseOracleNumberArguments::PrecisionAndScale(65, 30),
        );
        let family = ColumnTypeFamily::Unsupported("numeric(65,30)".into());
        for expression in [
            "NULL",
            "NULL::numeric",
            "(null::numeric)",
            "((NULL)::pg_catalog.numeric)::decimal",
            "NULL::sys_catalog.numeric",
        ] {
            assert!(
                parse_native(expression, &family, Some(&native)).is_none(),
                "{expression}"
            );
        }
        for expression in [
            "'NULL'",
            "'NULL'::numeric",
            "NULLIF(1,2)::numeric",
            "NULLABLE()",
            "NULL::numeric + 1",
            "NULL::custom_domain",
            "'1.25'::numeric",
            "1.25::numeric(10,1)",
        ] {
            assert_eq!(
                parse_native(expression, &family, Some(&native)).unwrap().kind(),
                &DefaultKind::DbGenerated(Some(expression.to_owned())),
                "{expression}",
            );
        }
        assert_eq!(
            parse_native(
                "'NULL'::text",
                &ColumnTypeFamily::String,
                Some(&KingbaseOracleType::VarChar2(Some(20)))
            )
            .unwrap()
            .kind(),
            &DefaultKind::Value(PrismaValue::String("NULL".to_owned())),
        );
        assert_eq!(
            parse_native("NULL::numeric", &family, None).unwrap().kind(),
            &DefaultKind::DbGenerated(Some("NULL::numeric".to_owned())),
        );
    }

    #[test]
    fn native_uuid_and_xml_literal_defaults_require_the_matching_column_type() {
        for (native, cast, value) in [
            (KingbaseOracleType::Uuid, "uuid", "123e4567-e89b-12d3-a456-426614174000"),
            (KingbaseOracleType::Xml, "xml", "<item name='test'>value</item>"),
        ] {
            let quoted = value.replace('\'', "''");
            for expression in [
                format!("'{quoted}'::{cast}"),
                format!("('{quoted}'::sys_catalog.{cast})"),
            ] {
                assert_eq!(
                    parse_native(&expression, &ColumnTypeFamily::String, Some(&native))
                        .unwrap()
                        .kind(),
                    &DefaultKind::Value(PrismaValue::String(value.to_owned())),
                );
                assert!(matches!(
                    parse_native(
                        &expression,
                        &ColumnTypeFamily::String,
                        Some(&KingbaseOracleType::VarChar2(Some(100)))
                    )
                    .unwrap()
                    .kind(),
                    DefaultKind::DbGenerated(_),
                ));
            }
            for expression in [
                format!("make_value()::{cast}"),
                format!("'{quoted}'::varchar(5)::{cast}"),
            ] {
                assert_eq!(
                    parse_native(&expression, &ColumnTypeFamily::String, Some(&native))
                        .unwrap()
                        .kind(),
                    &DefaultKind::DbGenerated(Some(expression.clone())),
                );
            }
        }
    }

    #[test]
    fn numeric_default_comparison_recognizes_oracle_casts_and_lossless_catalog_decorations() {
        for (model, catalog) in [
            ("CAST(1.25 AS NUMBER(10,1))", "1.25::numeric(10,1)"),
            ("(CAST(1.25 AS NUMBER(10,1)))", "(1.25::numeric(3,2))::numeric(10,1)"),
            ("CAST('1.25' AS NUMBER(10,1))", "1.25::numeric(10,1)"),
            ("CAST(-1.250 AS NUMBER(10,1))", "(-1.25::numeric(3,2))::numeric(10,1)"),
            ("CAST(1.25 AS NUMBER(10,2))", "1.25::numeric(3,2)::numeric(10,2)"),
            (
                "CAST(1e-30 AS NUMBER(31,30))",
                "0.000000000000000000000000000001::numeric",
            ),
            ("CAST(0.0012 AS NUMBER(2,4))", "0.0012::numeric"),
            ("CAST(0.12 AS NUMBER(2,4))", "0.12::numeric(4,4)"),
            ("CAST('0.12' AS NUMBER(2,4))", "0.12::numeric(4,4)"),
            ("CAST(0.12 AS NUMBER(38,50))", "0.12::numeric(50,50)"),
            ("CAST(0.12345 AS NUMBER(2,4))", "0.12345::numeric(4,4)"),
            (
                "CAST(CAST(1.25 AS NUMBER(10,1)) AS NUMBER(10,2))",
                "((1.25::numeric(3,2))::numeric(10,1))::numeric(10,2)",
            ),
        ] {
            assert!(numeric_default_expressions_match(model, catalog), "{model} / {catalog}");
            assert!(numeric_default_expressions_match(catalog, model), "{catalog} / {model}");
        }
    }

    #[test]
    fn numeric_default_comparison_preserves_semantic_casts_and_unknown_expressions() {
        for (previous, next) in [
            ("CAST(1.25 AS NUMBER(10,1))", "1.25::numeric(10,2)"),
            ("CAST(1.25 AS NUMBER(10,1))", "1.35::numeric(10,1)"),
            ("CAST(1.25 AS NUMBER(10,1))", "1.3::numeric(10,1)"),
            (
                "CAST(CAST(1.25 AS NUMBER(10,1)) AS NUMBER(10,2))",
                "1.25::numeric(10,2)",
            ),
            ("CAST(1000 AS NUMBER(3,0))", "1000::numeric(4,0)"),
            ("CAST(0.12345 AS NUMBER(2,4))", "0.12345::numeric(5,5)"),
            ("CAST(0.12345 AS NUMBER(2,4))", "0.1235::numeric(4,4)"),
            ("CAST(1.25 AS NUMBER)", "1.25::numeric"),
            ("1.25::NUMBER", "1.25::numeric"),
            ("CAST(1.25 AS NUMBER(10,-1))", "1.25::numeric(10,0)"),
            ("CAST(1.25 AS NUMBER(0,1))", "1.25::numeric(10,1)"),
            ("CAST(1.25 AS NUMBER(1001,1))", "1.25::numeric(10,1)"),
            ("CAST(1.25 AS public.NUMBER(10,1))", "1.25::numeric(10,1)"),
            ("CAST(1.25 AS \"NUMBER\"(10,1))", "1.25::numeric(10,1)"),
            ("CAST(1.25 AS BINARY_FLOAT)", "1.25::numeric"),
            ("CAST(1.25 AS NUMBER(10,1)) + 1", "1.25::numeric(10,1)"),
            ("CAST(1.25 AS NUMBER(10,1)); SELECT 1", "1.25::numeric(10,1)"),
            ("'1.25'", "1.25"),
            ("CAST('not a number' AS NUMBER(10,1))", "1.25::numeric(10,1)"),
        ] {
            assert!(
                !numeric_default_expressions_match(previous, next),
                "{previous} / {next}"
            );
            assert!(
                !numeric_default_expressions_match(next, previous),
                "{next} / {previous}"
            );
        }
    }

    #[test]
    fn character_and_negative_integer_defaults_normalize_only_equivalent_casts() {
        for native in [KingbaseOracleType::Char(Some(5)), KingbaseOracleType::NChar(Some(5))] {
            assert_eq!(
                parse_native("'test'::bpchar", &ColumnTypeFamily::String, Some(&native))
                    .unwrap()
                    .kind(),
                &DefaultKind::Value(PrismaValue::String("test".into()))
            );
            for expression in ["'abcdef'::bpchar(3)", "'test '::bpchar::text"] {
                assert_eq!(
                    parse_native(expression, &ColumnTypeFamily::String, Some(&native))
                        .unwrap()
                        .kind(),
                    &DefaultKind::DbGenerated(Some(expression.into()))
                );
            }
        }
        assert!(matches!(
            parse_native(
                "'test '::bpchar",
                &ColumnTypeFamily::String,
                Some(&KingbaseOracleType::VarChar2(Some(5)))
            )
            .unwrap()
            .kind(),
            DefaultKind::DbGenerated(_)
        ));
        for (family, expected) in [
            (ColumnTypeFamily::Int, PrismaValue::Int(-1)),
            (ColumnTypeFamily::BigInt, PrismaValue::BigInt(-1)),
        ] {
            for expression in ["'-1'::integer", "(-1)::int4", "'-1'::bigint::numeric"] {
                assert_eq!(
                    parse(expression, &family).unwrap().kind(),
                    &DefaultKind::Value(expected.clone())
                );
            }
            for expression in ["(1.6)::integer", "(-1)::numeric(2,0)"] {
                assert_eq!(
                    parse(expression, &family).unwrap().kind(),
                    &DefaultKind::DbGenerated(Some(expression.into()))
                );
            }
        }
    }
    #[test]
    fn floating_casts_only_normalize_at_the_target_precision() {
        for (expression, native) in [
            ("(1.23456789)::real", KingbaseOracleType::BinaryDouble),
            (
                "((1.23456789)::real)::double precision",
                KingbaseOracleType::BinaryDouble,
            ),
            ("(1.23456789)::double precision", KingbaseOracleType::BinaryFloat),
        ] {
            assert_eq!(
                parse_native(expression, &ColumnTypeFamily::Float, Some(&native))
                    .unwrap()
                    .kind(),
                &DefaultKind::DbGenerated(Some(expression.into()))
            );
        }
        for (expression, native) in [
            ("1.25::real", KingbaseOracleType::BinaryFloat),
            ("1.25::double precision", KingbaseOracleType::BinaryDouble),
        ] {
            assert!(matches!(
                parse_native(expression, &ColumnTypeFamily::Float, Some(&native))
                    .unwrap()
                    .kind(),
                DefaultKind::Value(_)
            ));
        }
    }
    #[test]
    fn null_is_a_token_not_an_identifier_prefix() {
        for null in ["NULL", "null::numeric", "(NULL)"] {
            assert!(parse(null, &ColumnTypeFamily::Int).is_none());
        }
        for expression in ["NULLIF(1,2)", "nullif(1,2)", "NULLIF(1,2)::numeric", "NULLABLE()"] {
            assert_eq!(
                parse(expression, &ColumnTypeFamily::Int).unwrap().kind(),
                &DefaultKind::DbGenerated(Some(expression.into()))
            );
        }
    }
    #[test]
    fn sequence_forms_and_unknown_functions_remain_distinct() {
        for expression in [
            "seq.NEXTVAL",
            "nextval('seq'::regclass)",
            "sys_catalog.nextval('seq'::regclass)",
            "pg_catalog.nextval(('seq'::text)::regclass)",
        ] {
            assert_eq!(
                parse(expression, &ColumnTypeFamily::Int).unwrap().kind(),
                &DefaultKind::Sequence("seq".into())
            );
        }
        assert!(sequence("custom.nextval('seq')").is_none());
        assert!(sequence("seq.NEXTVAL+1").is_none());
    }
    #[test]
    fn strings_and_binary_literals_keep_exact_values() {
        for (input, expected) in [
            ("'a''b\\'::varchar", "a'b\\"),
            ("q'[it's :ignored]'", "it's :ignored"),
            ("N'字'", "字"),
        ] {
            assert_eq!(
                parse(input, &ColumnTypeFamily::String).unwrap().kind(),
                &DefaultKind::Value(PrismaValue::String(expected.into()))
            );
        }
        for input in ["decoding('00abcd'::text,'hex'::text)", "HEXTORAW('00abcd')"] {
            assert_eq!(
                parse(input, &ColumnTypeFamily::Binary).unwrap().kind(),
                &DefaultKind::Value(PrismaValue::Bytes(vec![0, 0xab, 0xcd]))
            );
        }
        assert!(binary("decoding('xx','hex')").is_none());
    }
    #[test]
    fn typmod_casts_remain_expressions_instead_of_inventing_unrounded_constants() {
        for expression in [
            "(1.25)::numeric(2,1)",
            "'abcdef'::varchar(3)",
            "'1'::text::boolean::text",
            "('abcdef'::character(3 char))",
        ] {
            let family = if expression.contains("1.25") {
                ColumnTypeFamily::Decimal
            } else {
                ColumnTypeFamily::String
            };
            assert_eq!(
                parse(expression, &family).unwrap().kind(),
                &DefaultKind::DbGenerated(Some(expression.into()))
            );
        }
    }
}

#[cfg(test)]
mod added_native_type_defaults {
    use super::*;

    #[test]
    fn recognizes_matching_native_casts_without_losing_fixed_width_semantics() {
        for (native, cast) in [
            (KingbaseOracleType::CharByte(8), "bpcharbyte"),
            (KingbaseOracleType::VarChar2Byte(8), "varcharbyte"),
            (KingbaseOracleType::RowId, "\"rowid\""),
            (KingbaseOracleType::URowId(Some(4000)), "urowid"),
        ] {
            let expression = format!("'value'::{cast}");
            let parsed = parse_native(&expression, &ColumnTypeFamily::String, Some(&native)).unwrap();
            assert_eq!(
                parsed.kind(),
                &crate::DefaultKind::Value(PrismaValue::String("value".into()))
            );
        }
        for cast in ["bpcharbyte", "\"bpcharbyte\"", "sys.bpcharbyte"] {
            let expression = format!("'abc '::{cast}::text");
            let parsed = parse_native(
                &expression,
                &ColumnTypeFamily::String,
                Some(&KingbaseOracleType::CharByte(8)),
            )
            .unwrap();
            assert!(matches!(parsed.kind(), crate::DefaultKind::DbGenerated(Some(value)) if value == &expression));
        }
    }
}
