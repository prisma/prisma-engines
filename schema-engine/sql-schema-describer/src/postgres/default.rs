mod c_style_scalar_lists;
mod tokenize;

use crate::{ColumnType, ColumnTypeFamily, DefaultKind, DefaultValue};
use prisma_value::PrismaValue;
use tokenize::{tokenize, Token};

#[derive(Debug)]
struct Parser<'a> {
    input: &'a str,
    tokens: &'a [(Token, u32)],
    offset: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str, tokens: &'a [(Token, u32)]) -> Self {
        Parser {
            tokens,
            offset: 0,
            input,
        }
    }

    fn resolve_offset(&self, offset: usize) -> Option<(Token, &'a str)> {
        match (self.tokens.get(offset), self.tokens.get(offset + 1)) {
            (None, _) => None,
            (Some((tok, start)), None) => Some((*tok, &self.input[(*start as usize)..])),
            (Some((tok, start)), Some((_, end))) => Some((*tok, &self.input[(*start as usize)..*end as usize])),
        }
    }

    fn peek_token(&self) -> Option<Token> {
        self.next_non_whitespace().map(|(_, (tok, _))| tok)
    }

    fn next_token(&mut self) -> Option<Token> {
        let (next_offset, (token, _)) = self.next_non_whitespace()?;
        self.offset = next_offset + 1;
        Some(token)
    }

    fn next_non_whitespace(&self) -> Option<(usize, (Token, u32))> {
        let mut offset = self.offset;

        loop {
            match self.tokens.get(offset)? {
                (Token::Whitespace, _) => {
                    offset += 1;
                }
                other => break Some((offset, *other)),
            }
        }
    }

    #[must_use]
    fn expect(&mut self, expected_token: Token) -> Option<&'a str> {
        let (next_offset, (next_token, _start)) = self.next_non_whitespace()?;

        if next_token != expected_token {
            return None;
        }

        self.offset = next_offset + 1;

        Some(self.resolve_offset(next_offset)?.1)
    }

    /// `true` if all input tokens have been consumed.
    fn is_finished(&self) -> bool {
        self.offset >= self.tokens.len() - 1
    }
}

pub(super) fn get_default_value(default_string: &str, tpe: &ColumnType) -> Option<DefaultValue> {
    if default_string.trim().starts_with("NULL") {
        return None;
    }

    let tokens = tokenize(default_string);
    let mut parser = Parser::new(default_string, &tokens);

    if tpe.arity.is_list() {
        return Some(get_list_default_value(&mut parser, tpe));
    }

    let parser_fn = parser_for_family(&tpe.family);
    let parsed_default = parser_fn(&mut parser).filter(|_| parser.is_finished());

    Some(match parsed_default {
        Some(default_value) => default_value,
        None => DefaultValue::db_generated(default_string.to_owned()),
    })
}

fn parser_for_family(family: &ColumnTypeFamily) -> &'static dyn Fn(&mut Parser<'_>) -> Option<DefaultValue> {
    match family {
        ColumnTypeFamily::String | ColumnTypeFamily::Json => &parse_string_default,
        ColumnTypeFamily::Int | ColumnTypeFamily::BigInt => &parse_int_default,
        ColumnTypeFamily::Enum(_) => &parse_enum_default,
        ColumnTypeFamily::Float | ColumnTypeFamily::Decimal => &parse_float_default,
        ColumnTypeFamily::Boolean => &parse_bool_default,
        ColumnTypeFamily::DateTime => &parse_datetime_default,
        ColumnTypeFamily::Binary => &parse_binary_default,
        ColumnTypeFamily::Unsupported(_) | ColumnTypeFamily::Uuid => &parse_unsupported,
    }
}

fn parse_unsupported(_parser: &mut Parser<'_>) -> Option<DefaultValue> {
    None
}

fn parse_datetime_default(parser: &mut Parser<'_>) -> Option<DefaultValue> {
    match parser.peek_token()? {
        Token::Identifier => {
            let func_name = parser.expect(Token::Identifier)?;
            if let Some(Token::OpeningBrace) = parser.peek_token() {
                parser.expect(Token::OpeningBrace)?;
                parser.expect(Token::ClosingBrace)?;
            }

            eat_cast(parser)?;

            match func_name {
                name if name.eq_ignore_ascii_case("now") || name.eq_ignore_ascii_case("current_timestamp") => {
                    Some(DefaultValue::now())
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn parse_enum_default(parser: &mut Parser<'_>) -> Option<DefaultValue> {
    match parser.peek_token()? {
        Token::Identifier => {
            let s = parser.expect(Token::Identifier)?;
            eat_cast(parser)?;
            Some(DefaultValue::value(PrismaValue::Enum(s.to_owned())))
        }
        Token::StringLiteral | Token::CStyleStringLiteral => {
            let value = parse_string_value(parser)?;
            eat_cast(parser)?;
            Some(DefaultValue::value(PrismaValue::Enum(value)))
        }
        _ => None,
    }
}

fn parse_string_value(parser: &mut Parser<'_>) -> Option<String> {
    match parser.peek_token()? {
        Token::StringLiteral => {
            let s = parser.expect(Token::StringLiteral)?;
            let mut out = String::with_capacity(s.len() - 2); // exclude the quotes
            let mut chars = s[1..(s.len() - 1)].chars();

            loop {
                match chars.next() {
                    Some('\'') => {
                        assert!(chars.next() == Some('\'')); // invariant
                        out.push('\'');
                    }
                    Some(c) => {
                        out.push(c);
                    }
                    None => break,
                }
            }

            eat_cast(parser)?;

            Some(out)
        }
        Token::CStyleStringLiteral => {
            // Reference for CockroachDB: https://www.cockroachlabs.com/docs/stable/sql-constants.html#string-literals-with-character-escapes
            // Reference for Postgres: https://www.postgresql.org/docs/current/sql-syntax-lexical.html#SQL-SYNTAX-CONSTANTS
            // octal and hexadecimal escape sequences seem not to be returned by crdb in defaults,
            // so we do not try parsing them.
            let s = parser.expect(Token::CStyleStringLiteral)?;
            let mut out = String::with_capacity(s.len() - 3); // exclude the quotes and E
            let mut chars = s[2..(s.len() - 1)].chars().peekable();

            loop {
                match chars.next() {
                    Some('\\') => {
                        let next_char = chars.next().expect("invariant");

                        match next_char {
                            'a' => {
                                out.push('\u{7}');
                            }
                            'b' => {
                                out.push('\u{8}');
                            }
                            'v' => {
                                out.push('\u{11}');
                            }
                            'f' => {
                                out.push('\u{12}');
                            }
                            'n' => {
                                out.push('\n');
                            }
                            'r' => {
                                out.push('\r');
                            }
                            't' => {
                                out.push('\t');
                            }
                            'u' => {
                                // take 4
                                let mut codepoint = 0u32;
                                for i in 0..4 {
                                    let nibble_offset = 3 - i;
                                    // expect crdb to return valid codepoints
                                    let next_digit = chars.next().unwrap().to_digit(16).unwrap();
                                    codepoint += next_digit << (nibble_offset * 4);
                                }
                                out.push(char::from_u32(codepoint).unwrap()); // assume crdb returns valid codepoints
                            }
                            'U' => {
                                // take 8
                                let mut codepoint = 0u32;
                                for i in 0..8 {
                                    let nibble_offset = 7 - i;
                                    // expect crdb to return valid codepoints
                                    let next_digit = chars.next().unwrap().to_digit(16).unwrap();
                                    codepoint += next_digit << (nibble_offset * 4);
                                }
                                out.push(char::from_u32(codepoint).unwrap()); // assume crdb returns valid codepoints
                            }
                            _ => out.push(next_char),
                        }
                    }
                    Some(c) => {
                        out.push(c);
                    }
                    None => break,
                }
            }

            eat_cast(parser)?;

            Some(out)
        }
        _ => None,
    }
}

fn parse_string_default(parser: &mut Parser<'_>) -> Option<DefaultValue> {
    let out = parse_string_value(parser)?;
    Some(DefaultValue::value(out))
}

fn parse_identifier(parser: &mut Parser<'_>) -> Option<String> {
    match parser.peek_token()? {
        Token::DoubleQuotedIdentifier => {
            let s = parser.expect(Token::DoubleQuotedIdentifier)?;
            Some(parse_double_quoted_string_contents(s))
        }
        Token::Identifier => {
            let s = parser.expect(Token::Identifier)?;
            Some(s.to_owned())
        }
        _ => None,
    }
}

fn parse_double_quoted_string_contents(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s[1..(s.len() - 1)].chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                out.push(chars.next().unwrap());
            }
            other => {
                out.push(other);
            }
        }
    }

    out
}

fn parse_int_default(parser: &mut Parser<'_>) -> Option<DefaultValue> {
    match parser.peek_token()? {
        Token::Digits => {
            let s = parser.expect(Token::Digits)?;
            let parsed: i64 = s.parse().ok()?;
            Some(DefaultValue::value(parsed))
        }
        Token::Minus => {
            parser.expect(Token::Minus)?; // consume
            let s = parser.expect(Token::Digits)?;
            let parsed: i64 = s.parse().ok()?;
            Some(DefaultValue::value(-parsed))
        }
        Token::StringLiteral => {
            let contents = parse_string_value(parser)?;
            let parsed: i64 = contents.parse().ok()?;
            Some(DefaultValue::value(parsed))
        }
        Token::Identifier => {
            let s = parser.expect(Token::Identifier)?;

            if s.eq_ignore_ascii_case("unique_rowid") {
                parser.expect(Token::OpeningBrace)?;
                parser.expect(Token::ClosingBrace)?;
                Some(DefaultValue::unique_rowid())
            } else if s.eq_ignore_ascii_case("nextval") {
                parser.expect(Token::OpeningBrace)?;

                // Example: nextval(('"third_Sequence"'::text)::regclass)
                if let Some(Token::OpeningBrace) = parser.peek_token() {
                    parser.expect(Token::OpeningBrace)?;
                }

                let sequence_name_string = match parser.peek_token() {
                    Some(Token::StringLiteral) | Some(Token::CStyleStringLiteral) => parse_string_value(parser)?,
                    _ => return None,
                };

                let sequence_name = {
                    let tokens = tokenize(&sequence_name_string);
                    let mut parser = Parser::new(&sequence_name_string, &tokens);
                    let first_ident = parse_identifier(&mut parser)?;
                    // Maybe the first identifier is the schema name, we have to see if there is a
                    // dot followed by a second identifier.
                    if let Some(Token::Dot) = parser.peek_token() {
                        parser.expect(Token::Dot)?;
                        parse_identifier(&mut parser)?
                    } else {
                        first_ident
                    }
                };

                loop {
                    if let Token::ClosingBrace = parser.next_token()? {
                        break;
                    }
                }

                eat_cast(parser)?;

                if let Some(Token::ClosingBrace) = parser.peek_token() {
                    parser.expect(Token::ClosingBrace)?;
                }

                eat_cast(parser)?;

                Some(DefaultValue::sequence(sequence_name))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn parse_float_default(parser: &mut Parser<'_>) -> Option<DefaultValue> {
    fn parse_float_default_inner(parser: &mut Parser<'_>) -> Option<DefaultValue> {
        let mut open_brace = false;

        match parser.peek_token()? {
            Token::OpeningBrace => {
                parser.expect(Token::OpeningBrace)?;
                open_brace = true;
            }
            Token::StringLiteral | Token::CStyleStringLiteral => {
                let parsed_string = parse_string_value(parser)?;
                let tokens = tokenize(&parsed_string);
                let mut string_parser = Parser::new(&parsed_string, &tokens);
                return parse_float_default_inner(&mut string_parser);
            }
            _ => (),
        }

        let value: bigdecimal::BigDecimal = {
            let sign = if let Token::Minus = parser.peek_token()? {
                parser.expect(Token::Minus)?
            } else {
                ""
            };
            let integer_part = parser.expect(Token::Digits)?;

            // The fractional part is optional
            let (dot, fractional_part) = if let Some(Token::Dot) = parser.peek_token() {
                parser.expect(Token::Dot)?;
                let digits = parser.expect(Token::Digits)?;
                (".", digits)
            } else {
                ("", "")
            };

            let complete = format!("{sign}{integer_part}{dot}{fractional_part}");
            complete.parse().ok()?
        };

        if open_brace {
            parser.expect(Token::ClosingBrace)?;
        }

        Some(DefaultValue::value(PrismaValue::Float(value)))
    }

    let value = parse_float_default_inner(parser)?;
    eat_cast(parser)?;
    Some(value)
}

fn parse_bool_default(parser: &mut Parser<'_>) -> Option<DefaultValue> {
    let s = parser.expect(Token::Identifier)?;

    let bool_value = if s.eq_ignore_ascii_case("t") || s.eq_ignore_ascii_case("true") {
        true
    } else if s.eq_ignore_ascii_case("f") || s.eq_ignore_ascii_case("false") {
        false
    } else {
        return None;
    };

    Some(DefaultValue::value(bool_value))
}

fn parse_binary_default(parser: &mut Parser<'_>) -> Option<DefaultValue> {
    let value = parse_string_value(parser)?;
    if !value.starts_with("\\x") || !value.is_ascii() {
        return None;
    }

    let hex_bytes = &value[2..];
    let mut bytes = hex_bytes.as_bytes().chunks_exact(2);
    let mut decoded_bytes = Vec::with_capacity(value.len() / 2);

    for nibbles in &mut bytes {
        let high_nibble = &nibbles[0];
        let low_nibble = &nibbles[1];
        let high_nibble: u8 = u8::from_str_radix(std::str::from_utf8(&[*high_nibble]).unwrap(), 16).unwrap() << 4;
        let low_nibble: u8 = u8::from_str_radix(std::str::from_utf8(&[*low_nibble]).unwrap(), 16).unwrap();
        decoded_bytes.push(high_nibble | low_nibble);
    }

    if !bytes.remainder().is_empty() {
        return None;
    }

    Some(DefaultValue::value(PrismaValue::Bytes(decoded_bytes)))
}

fn parse_array_constructor(parser: &mut Parser<'_>, tpe: &ColumnTypeFamily) -> Option<Vec<PrismaValue>> {
    let mut values = Vec::new();
    let parse_fn = parser_for_family(tpe);

    let _ = parser.expect(Token::OpeningBrace);

    let kw = parser.expect(Token::Identifier)?;
    if !kw.eq_ignore_ascii_case("array") {
        return None;
    }

    parser.expect(Token::OpeningSquareBracket)?;

    loop {
        match parser.peek_token() {
            None => return None, // missing closing bracket
            Some(Token::ClosingSquareBracket) => {
                parser.expect(Token::ClosingSquareBracket)?;
                break;
            }
            Some(_) => {
                values.push(parse_fn(parser)?);
            }
        }

        // Now the comma between values or the end of the array.
        match parser.next_token() {
            None => return None, // missing closing bracket
            Some(Token::ClosingSquareBracket) => break,
            Some(Token::Comma) => (), // continue
            Some(_) => return None,   // unexpected token
        }
    }

    let mut extracted_values = Vec::with_capacity(values.len());

    for value in values {
        match value.kind {
            DefaultKind::Value(val) => extracted_values.push(val),
            _ => return None, // non-literal in array default
        }
    }

    Some(extracted_values)
}

fn get_list_default_value(parser: &mut Parser<'_>, tpe: &ColumnType) -> DefaultValue {
    let values = match parser.peek_token() {
        Some(Token::CStyleStringLiteral) | Some(Token::StringLiteral) => {
            parse_string_value(parser).and_then(|value| c_style_scalar_lists::parse_array_literal(&value, tpe))
        }
        Some(Token::Identifier) | Some(Token::OpeningBrace) => parse_array_constructor(parser, &tpe.family),
        _ => None,
    };

    values
        .map(|values| DefaultValue::value(PrismaValue::List(values)))
        .unwrap_or_else(|| DefaultValue::db_generated(parser.input.to_owned()))
}

/// Some(()) on valid cast or absence of cast. None if we can't make sense of the input.
#[must_use]
fn eat_cast(parser: &mut Parser<'_>) -> Option<()> {
    match parser.peek_token() {
        Some(Token::CastOperator) => {
            parser.expect(Token::CastOperator)?;
        }
        _ => return Some(()),
    }

    // One or more identifiers
    //
    // TIMESTAMP WITH TIME ZONE (4)[]
    // ^^^^^^^^^^^^^^^^^^^^^^^^
    //
    // or
    //
    // "color"
    loop {
        match parser.peek_token() {
            Some(Token::DoubleQuotedIdentifier) => {
                parser.expect(Token::DoubleQuotedIdentifier)?;
            }
            Some(Token::Identifier) => {
                parser.expect(Token::Identifier)?;
            }
            Some(Token::Dot) => {
                // schema-qualified types
                // e.g. my-schema.color
                parser.expect(Token::Dot)?;
            }
            _ => break,
        }
    }

    // Optional precision
    // TIMESTAMP WITH TIME ZONE (4)[]
    //                          ^^^
    if let Some(Token::OpeningBrace) = parser.peek_token() {
        loop {
            if let Token::ClosingBrace = parser.next_token()? {
                break;
            }
        }
    }

    // Optional array modifier
    // TIMESTAMP WITH TIME ZONE (4)[]
    //                             ^^
    if let Some(Token::OpeningSquareBracket) = parser.peek_token() {
        parser.expect(Token::OpeningSquareBracket)?;
        parser.expect(Token::ClosingSquareBracket)?;
    }

    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;

    #[test]
    fn parse_string_array_default() {
        let input = "ARRAY['abc', 'def']::text[]";
        let tokens = tokenize(input);
        let mut parser = Parser::new(input, &tokens);

        let out = parse_array_constructor(&mut parser, &ColumnTypeFamily::String);

        let expected = expect![[r#"
            Some(
                [
                    String(
                        "abc",
                    ),
                    String(
                        "def",
                    ),
                ],
            )
        "#]];

        expected.assert_debug_eq(&out);
    }

    #[test]
    fn parse_enum_array_default() {
        let input = "ARRAY['RED'::color, 'GREEN'::color]";
        let tokens = tokenize(input);
        let mut parser = Parser::new(input, &tokens);

        let out = parse_array_constructor(&mut parser, &ColumnTypeFamily::Enum(crate::EnumId(0))).unwrap();

        let expected = expect![[r#"
            [
                Enum(
                    "RED",
                ),
                Enum(
                    "GREEN",
                ),
            ]
        "#]];

        expected.assert_debug_eq(&out);
    }

    #[test]
    fn parse_enum_array_default_with_quotes() {
        let input = r#"ARRAY['RED'::"color", 'GREEN'::"color"]"#;
        let tokens = tokenize(input);
        let mut parser = Parser::new(input, &tokens);

        let out = parse_array_constructor(&mut parser, &ColumnTypeFamily::Enum(crate::EnumId(0))).unwrap();

        let expected = expect![[r#"
            [
                Enum(
                    "RED",
                ),
                Enum(
                    "GREEN",
                ),
            ]
        "#]];

        expected.assert_debug_eq(&out);
    }

    #[test]
    fn parse_int_array_default() {
        let input = "ARRAY[9, 12999, '-4'::integer, 0, 1249849]";
        let tokens = tokenize(input);
        let mut parser = Parser::new(input, &tokens);

        let out = parse_array_constructor(&mut parser, &ColumnTypeFamily::Int).unwrap();

        let expected = expect![[r#"
            [
                Int(
                    9,
                ),
                Int(
                    12999,
                ),
                Int(
                    -4,
                ),
                Int(
                    0,
                ),
                Int(
                    1249849,
                ),
            ]
        "#]];

        expected.assert_debug_eq(&out);
    }

    #[test]
    fn parse_decimal_array_default() {
        let input = "ARRAY[121.10299000124800000001::numeric(65,30), 0.4::numeric(65,30), 1.1::numeric(65,30), '-68.0'::numeric(65,30)]";
        let tokens = tokenize(input);
        let mut parser = Parser::new(input, &tokens);

        let out = parse_array_constructor(&mut parser, &ColumnTypeFamily::Decimal).unwrap();

        let expected = expect![[r#"
            [
                Float(
                    BigDecimal("121.10299000124800000001"),
                ),
                Float(
                    BigDecimal("0.4"),
                ),
                Float(
                    BigDecimal("1.1"),
                ),
                Float(
                    BigDecimal("-68.0"),
                ),
            ]
        "#]];

        expected.assert_debug_eq(&out);
    }

    #[test]
    fn parse_empty_varchar_array_default() {
        let input = "(ARRAY[]::character varying[])::character varying(10)[]";
        let tokens = tokenize(input);
        let mut parser = Parser::new(input, &tokens);

        let out = parse_array_constructor(&mut parser, &ColumnTypeFamily::String);

        let expected = expect![[r#"
            Some(
                [],
            )
        "#]];

        expected.assert_debug_eq(&out);
    }

    #[test]
    fn postgres_is_sequence_works() {
        let assert_is_sequence = |default_str: &str, expected_sequence: &str| {
            let parsed_default = get_default_value(
                default_str,
                &ColumnType::pure(ColumnTypeFamily::Int, crate::ColumnArity::Required),
            );
            let known_default = parsed_default.unwrap();
            assert_eq!(known_default.as_sequence().unwrap(), expected_sequence);
        };

        assert_is_sequence(r#"nextval('first_sequence'::regclass)"#, "first_sequence");

        assert_is_sequence(r#"nextval('schema_name.second_sequence'::regclass)"#, "second_sequence");

        assert_is_sequence(r#"nextval('"third_Sequence"'::regclass)"#, "third_Sequence");
        assert_is_sequence(
            r#"nextval('"schema_Name"."fourth_Sequence"'::regclass)"#,
            "fourth_Sequence",
        );

        assert_is_sequence(r#"nextval(('fifth_sequence'::text)::regclass)"#, "fifth_sequence");
        let non_autoincrement = r#"string_default_named_seq"#;
        assert!(get_default_value(
            non_autoincrement,
            &ColumnType::pure(ColumnTypeFamily::Int, crate::ColumnArity::Required)
        )
        .unwrap()
        .is_db_generated());
    }
}
