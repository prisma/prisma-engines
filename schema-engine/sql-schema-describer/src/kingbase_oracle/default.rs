//! Oracle literals and the documented KES decompiler forms. Unknown expressions
//! remain dbgenerated; their original spelling is never rewritten or executed.
use crate::{ColumnTypeFamily, DefaultValue, PrismaValue};
use psl::builtin_connectors::KingbaseOracleType;
use regex::Regex;
use std::{str::FromStr, sync::LazyLock};

#[cfg(test)]
pub(super) fn parse(input: &str, family: &ColumnTypeFamily) -> Option<DefaultValue> {
    parse_inner(input, family, None, None, false)
}

pub(super) fn parse_native(
    input: &str,
    family: &ColumnTypeFamily,
    native: Option<&KingbaseOracleType>,
) -> Option<DefaultValue> {
    let floating_cast = match native {
        Some(KingbaseOracleType::BinaryFloat | KingbaseOracleType::Float(Some(1..=24))) => Some("real"),
        Some(KingbaseOracleType::BinaryDouble | KingbaseOracleType::Float(None | Some(25..=53))) => {
            Some("double precision")
        }
        _ => None,
    };
    let fixed_character = matches!(native, Some(KingbaseOracleType::Char(_) | KingbaseOracleType::NChar(_)));
    parse_inner(input, family, None, floating_cast, fixed_character)
}

pub(super) fn parse_enum(input: &str, family: &ColumnTypeFamily, namespace: &str, name: &str) -> Option<DefaultValue> {
    parse_inner(input, family, Some((namespace, name)), None, false)
}

fn parse_inner(
    input: &str,
    family: &ColumnTypeFamily,
    enm: Option<(&str, &str)>,
    floating_cast: Option<&str>,
    fixed_character: bool,
) -> Option<DefaultValue> {
    let allowed = |tail: &str| harmless_casts(tail, family, enm, floating_cast, fixed_character);
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
) -> bool {
    let quote = |name: &str| format!("\"{}\"", name.replace('"', "\"\""));
    let mut names: Vec<String> = match family {
        ColumnTypeFamily::String => {
            let mut casts = vec!["text", "varchar", "character varying"];
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
    }
    .into_iter()
    .flat_map(|name| {
        [
            name.to_owned(),
            format!("sys_catalog.{name}"),
            format!("pg_catalog.{name}"),
        ]
    })
    .collect();
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
        let is_fixed = matches!(name.rsplit('.').next(), Some("bpchar" | "character"));
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
