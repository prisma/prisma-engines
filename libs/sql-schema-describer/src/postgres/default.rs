use crate::{ColumnType, ColumnTypeFamily, DefaultKind, DefaultValue};
use prisma_value::PrismaValue;
use std::iter::Peekable;

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
enum Token {
    Comma,
    OpeningBrace,         // (
    ClosingBrace,         // )
    OpeningSquareBracket, // [
    ClosingSquareBracket, // ]
    Minus,                // -
    Dot,                  // .
    EscapedDoubleQuote,   // \"
    EscapedBackslash,     // \\
    CastOperator,         // ::
    DoubleQuote,          // "
    StringLiteral,        // '...'
    CStyleStringLiteral,  // E'...'
    Digits,               // sequence of digits without dots or minus
    Identifier,
    UnterminatedStringLiteral,
    Whitespace,
    BadToken,
}

fn tokenize(default_string: &str) -> Vec<(Token, u32)> {
    let mut char_indices = default_string.char_indices().map(|(idx, c)| (idx as u32, c)).peekable();
    let mut out = Vec::new();

    loop {
        match char_indices.next() {
            None => return out,
            Some((start, ',')) => out.push((Token::Comma, start)),
            Some((start, '"')) => out.push((Token::DoubleQuote, start)),
            Some((start, '(')) => out.push((Token::OpeningBrace, start)),
            Some((start, ')')) => out.push((Token::ClosingBrace, start)),
            Some((start, '[')) => out.push((Token::OpeningSquareBracket, start)),
            Some((start, ']')) => out.push((Token::ClosingSquareBracket, start)),
            Some((start, '-')) => out.push((Token::Minus, start)),
            Some((start, '.')) => out.push((Token::Dot, start)),
            Some((start, ':')) => match char_indices.peek() {
                Some((_, ':')) => {
                    char_indices.next();
                    out.push((Token::CastOperator, start))
                }
                None | Some(_) => {
                    out.push((Token::BadToken, start));
                    return out;
                }
            },
            Some((start, '\\')) => match char_indices.peek() {
                Some((_, '"')) => {
                    char_indices.next();
                    out.push((Token::EscapedDoubleQuote, start))
                }
                Some((_, '\\')) => {
                    char_indices.next();
                    out.push((Token::EscapedBackslash, start))
                }
                None | Some(_) => {
                    out.push((Token::BadToken, start));
                    return out;
                }
            },
            Some((start, c)) if c.is_ascii_digit() => loop {
                match char_indices.peek() {
                    Some((_, c)) if c.is_ascii_digit() => {
                        char_indices.next();
                    }
                    None | Some(_) => {
                        out.push((Token::Digits, start));
                        break;
                    }
                }
            },
            Some((start, c)) if c == 'E' || c == 'e' => match char_indices.peek() {
                Some((_, '\'')) => {
                    // C-style string.
                    char_indices.next();

                    loop {
                        match char_indices.next() {
                            Some((_, '\\')) => match char_indices.next() {
                                None => {
                                    out.push((Token::UnterminatedStringLiteral, start));
                                    break;
                                }
                                Some(_) => {
                                    // consume
                                }
                            },
                            Some((_, '\'')) => {
                                out.push((Token::CStyleStringLiteral, start));
                                break;
                            }
                            Some(_) => {
                                // consume
                            }
                            None => {
                                out.push((Token::UnterminatedStringLiteral, start));
                                break;
                            }
                        }
                    }
                }
                Some((_, c)) if c.is_ascii_alphanumeric() => {
                    char_indices.next();
                    // identifier
                    loop {
                        match char_indices.peek() {
                            Some((_, c)) if c.is_ascii_alphanumeric() || *c == '_' => {
                                char_indices.next();
                            }
                            Some((_, _)) | None => {
                                out.push((Token::Identifier, start));
                                break;
                            }
                        }
                    }
                }
                None | Some(_) => {
                    out.push((Token::Identifier, start));
                }
            },
            Some((start, '\'')) => loop {
                match char_indices.next() {
                    None => {
                        out.push((Token::UnterminatedStringLiteral, start));
                        return out;
                    }
                    Some((_, '\'')) => match char_indices.peek() {
                        Some((_, '\'')) => {
                            char_indices.next();
                        }
                        None | Some(_) => {
                            out.push((Token::StringLiteral, start));
                            break;
                        }
                    },
                    Some((_, _)) => (),
                }
            },
            Some((start, c)) if c.is_ascii_whitespace() => loop {
                match char_indices.peek() {
                    Some((_idx, c)) if c.is_ascii_whitespace() => {
                        char_indices.next();
                    }
                    None | Some(_) => {
                        out.push((Token::Whitespace, start));
                        break;
                    }
                }
            },
            Some((start, c)) if c.is_ascii_alphabetic() => loop {
                match char_indices.peek() {
                    Some((_idx, c)) if c.is_ascii_alphanumeric() || *c == '_' => {
                        char_indices.next();
                    }
                    None | Some(_) => {
                        out.push((Token::Identifier, start));
                        break;
                    }
                }
            },
            Some((start, _)) => out.push((Token::BadToken, start)),
        }
    }
}

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
        self.peek().map(|(t, _)| t)
    }

    fn peek(&self) -> Option<(Token, &'a str)> {
        let mut offset = self.offset;

        loop {
            match self.resolve_offset(offset)? {
                (Token::Whitespace, _) => {
                    offset += 1;
                }
                other => break Some(other),
            }
        }
    }

    fn next(&mut self) -> Option<(Token, &'a str)> {
        loop {
            let token = self.resolve_offset(self.offset)?;
            self.offset += 1;

            if let Token::Whitespace = token.0 {
                continue;
            }

            break Some(token);
        }
    }

    #[must_use]
    fn expect(&mut self, expected_token: Token) -> Option<&'a str> {
        let (token, s) = self.resolve_offset(self.offset)?;

        if token != expected_token {
            return None;
        }

        self.offset += 1;

        Some(s)
    }

    /// Expect all input to have been consumed. Ignores whitespace.
    #[must_use]
    fn expect_consumed(&mut self) -> Option<()> {
        loop {
            match self.next() {
                Some((Token::Whitespace, _)) => (),
                Some(_) => break None,
                None => break Some(()),
            }
        }
    }
}

pub(super) fn get_default_value(default_string: &str, tpe: &ColumnType) -> Option<DefaultValue> {
    if default_string.starts_with("NULL") {
        return None;
    }

    let tokens = tokenize(default_string);
    let mut parser = Parser::new(default_string, &tokens);

    if tpe.arity.is_list() {
        return get_list_default_value(&mut parser, tpe);
    }

    Some(match &tpe.family {
        ColumnTypeFamily::Int | ColumnTypeFamily::BigInt => match parse_int_default(&mut parser) {
            Some(default_value) => default_value,
            None => {
                // TODO: use the parser for sequence defaults too
                // if let Some(seq) = is_sequence(&default_string, sequences) {
                //     DefaultValue::sequence(seq)
                // } else {
                DefaultValue::db_generated(default_string)
                // }
            }
        },
        ColumnTypeFamily::Float | ColumnTypeFamily::Decimal => match parse_float_default(&mut parser) {
            Some(float_value) => float_value,
            None => DefaultValue::db_generated(default_string),
        },
        ColumnTypeFamily::Boolean => match parse_bool_default(&mut parser) {
            Some(bool_value) => bool_value,
            None => DefaultValue::db_generated(default_string),
        },
        ColumnTypeFamily::String | ColumnTypeFamily::Json => match parse_string_default(&mut parser) {
            Some(string_default) => string_default,
            None => DefaultValue::db_generated(default_string),
        },
        ColumnTypeFamily::DateTime => match parse_datetime_default(&mut parser) {
            Some(default) => default,
            None => DefaultValue::db_generated(default_string),
        },
        // // JSON/JSONB defaults come in the '{}'::jsonb form.
        // ColumnTypeFamily::Json => unsuffix_default_literal(&default_string, &[data_type, &tpe.full_data_type])
        //     .map(|default| DefaultValue::value(PrismaValue::Json(unquote_string(&default))))
        //     .unwrap_or_else(move || DefaultValue::db_generated(default_string)),
        ColumnTypeFamily::Enum(_enum_name) => match parse_enum_default(&mut parser) {
            Some(default) => default,
            None => DefaultValue::db_generated(default_string),
        },
        ColumnTypeFamily::Uuid | ColumnTypeFamily::Binary | ColumnTypeFamily::Unsupported(_) => {
            DefaultValue::db_generated(default_string)
        }
    })
}

fn parse_datetime_default(parser: &mut Parser) -> Option<DefaultValue> {
    let func_name = parser.expect(Token::Identifier)?;
    parser.expect(Token::OpeningBrace)?;
    parser.expect(Token::ClosingBrace)?;

    match func_name {
        name if name.eq_ignore_ascii_case("now") || name.eq_ignore_ascii_case("current_timestamp") => {
            Some(DefaultValue::now())
        }
        _ => None,
    }
}

fn parse_enum_default(parser: &mut Parser) -> Option<DefaultValue> {
    match parser.peek()? {
        (Token::Identifier, s) => {
            parser.next(); // consume
            Some(DefaultValue::value(PrismaValue::Enum(s.to_owned())))
        }
        (Token::StringLiteral, _) | (Token::CStyleStringLiteral, _) => {
            let value = parse_string_value(parser)?;
            Some(DefaultValue::value(PrismaValue::Enum(value)))
        }
        _ => None,
    }
}

fn parse_string_value(parser: &mut Parser<'_>) -> Option<String> {
    match parser.next() {
        Some((Token::StringLiteral, s)) => {
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

            Some(out)
        }
        Some((Token::CStyleStringLiteral, s)) => {
            let mut out = String::with_capacity(s.len() - 3); // exclude the quotes and E
            let mut chars = s[2..(s.len() - 1)].chars();

            loop {
                match chars.next() {
                    Some('\\') => {
                        let next_char = chars.next().expect("invariant");
                        match next_char {
                            'n' => {
                                out.push('\n');
                            }
                            'r' => {
                                out.push('\r');
                            }
                            't' => {
                                out.push('\t');
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

            Some(out)
        }
        _ => None,
    }
}

fn parse_string_default(parser: &mut Parser<'_>) -> Option<DefaultValue> {
    let out = parse_string_value(parser)?;
    Some(DefaultValue::value(out))
}

fn parse_identifier<'a>(parser: &mut Parser<'a>) -> Option<&'a str> {
    match parser.next()? {
        (Token::DoubleQuote, _) => {
            let s = parser.expect(Token::Identifier)?;
            parser.expect(Token::DoubleQuote)?;
            Some(s)
        }
        (Token::Identifier, s) => Some(s),
        _ => None,
    }
}

fn parse_int_default(parser: &mut Parser<'_>) -> Option<DefaultValue> {
    match parser.peek() {
        Some((Token::Digits, s)) => {
            parser.next()?; // consume
            let parsed: i64 = s.parse().ok()?;
            Some(DefaultValue::value(parsed))
        }
        Some((Token::Minus, _)) => {
            parser.next()?; // consume
            let s = parser.expect(Token::Digits)?;
            let parsed: i64 = s.parse().ok()?;
            Some(DefaultValue::value(-parsed))
        }
        Some((Token::StringLiteral, _)) => {
            let contents = parse_string_value(parser)?;
            let parsed: i64 = contents.parse().ok()?;
            Some(DefaultValue::value(parsed))
        }
        Some((Token::Identifier, s)) if s.eq_ignore_ascii_case("unique_rowid") => {
            parser.next()?; // consume
            parser.expect(Token::OpeningBrace)?;
            parser.expect(Token::ClosingBrace)?;
            return Some(DefaultValue::unique_rowid());
        }
        Some((Token::Identifier, s)) if s.eq_ignore_ascii_case("nextval") => {
            parser.next()?; // consume
            parser.expect(Token::OpeningBrace)?;

            let sequence_name_string = match parser.peek() {
                Some((Token::StringLiteral, _)) | Some((Token::CStyleStringLiteral, _)) => parse_string_value(parser)?,
                _ => return None,
            };

            let sequence_name = {
                let tokens = tokenize(&sequence_name_string);
                let mut parser = Parser::new(&sequence_name_string, &tokens);
                parse_identifier(&mut parser)?.to_owned()
            };

            loop {
                match parser.next()? {
                    (Token::ClosingBrace, _) => break,
                    _ => (),
                }
            }

            return Some(DefaultValue::sequence(sequence_name));
        }
        _ => None,
    }
}

fn parse_float_default(parser: &mut Parser) -> Option<DefaultValue> {
    let mut open_brace = false;

    if let Token::OpeningBrace = parser.peek_token()? {
        open_brace = true;
        parser.next()?;
    }

    let value = todo!();

    if open_brace {
        parser.expect(Token::ClosingBrace)?;
    }

    match parser.next() {
        Some((Token::Digits, s)) => {
            parser.expect(Token::Dot)?;
            parser.expect(Token::Digits)?;
            parser.expect_consumed()?;
            let parsed: bigdecimal::BigDecimal = s.parse().ok()?;
            todo!("wrong");
            Some(DefaultValue::new(crate::DefaultKind::Value(PrismaValue::Float(parsed))))
        }
        Some((Token::Minus, _)) => {
            let s = parser.expect(Token::Digits)?;
            parser.expect(Token::Dot)?;
            parser.expect(Token::Digits)?;
            parser.expect_consumed()?;
            let parsed: i64 = s.parse().ok()?;
            todo!("wrong");
            Some(DefaultValue::value(-parsed))
        }
        Some((Token::StringLiteral, s)) => {
            let parsed: bigdecimal::BigDecimal = s[1..(s.len() - 2)].parse().ok()?;
            Some(DefaultValue::new(crate::DefaultKind::Value(PrismaValue::Float(parsed))))
        }
        _ => None,
    }
}

fn parse_bool_default(parser: &mut Parser) -> Option<DefaultValue> {
    let s = parser.expect(Token::Identifier)?;
    let b: bool = s.parse().ok()?;
    Some(DefaultValue::value(b))
}

fn parse_array_constructor(parser: &mut Parser<'_>, tpe: &ColumnTypeFamily) -> Option<Vec<PrismaValue>> {
    let mut values = Vec::new();
    let parse_fn: &dyn Fn(&mut Parser<'_>) -> Option<DefaultValue> = match tpe {
        ColumnTypeFamily::String | ColumnTypeFamily::Json => &parse_string_default,
        ColumnTypeFamily::Int | ColumnTypeFamily::BigInt => &parse_int_default,
        ColumnTypeFamily::Enum(_) => &parse_enum_default,
        ColumnTypeFamily::Float | ColumnTypeFamily::Decimal => &parse_float_default,
        ColumnTypeFamily::Boolean => &parse_bool_default,
        ColumnTypeFamily::DateTime => &parse_datetime_default,
        ColumnTypeFamily::Unsupported(_) | ColumnTypeFamily::Binary | ColumnTypeFamily::Uuid => return None,
    };

    let kw = parser.expect(Token::Identifier)?;
    if !kw.eq_ignore_ascii_case("array") {
        return None;
    }

    parser.expect(Token::OpeningSquareBracket)?;

    'outer: loop {
        match parser.peek() {
            None => return None, // missing closing bracket
            Some((Token::ClosingSquareBracket, _)) => {
                parser.next(); // consume
                break;
            }
            Some(_) => {
                values.push(parse_fn(parser)?);
            }
        }

        // Eat remaining tokens until the next comma.
        loop {
            // Now the comma between values
            match parser.next() {
                None => return None, // missing closing bracket
                Some((Token::ClosingSquareBracket, _)) => break 'outer,
                Some((Token::Comma, _)) => break,
                Some(_) => (),
            }
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

fn eat_spaces(chars: &mut Peekable<impl Iterator<Item = (usize, char)>>) {
    loop {
        match chars.peek() {
            Some((_, ' ')) => {
                chars.next();
            }
            _ => break,
        }
    }
}

fn get_list_default_value(parser: &mut Parser<'_>, tpe: &ColumnType) -> Option<DefaultValue> {
    match parse_array_literal(parser.input)
        .map(|(values, _)| values)
        .or_else(|| parse_array_constructor(parser, &tpe.family))
    {
        Some(values) => Some(DefaultValue::value(PrismaValue::List(values))),
        None => Some(DefaultValue::db_generated(parser.input)),
    }
}

// /// Returns the name of the sequence in the schema that the default value matches if it is drawn
// /// from one of them.
// fn is_sequence<'a>(value: &str, sequences: &'a [Sequence]) -> Option<&'a str> {
//     static AUTOINCREMENT_REGEX: Lazy<Regex> = Lazy::new(|| {
//         Regex::new(
//             r#"nextval\((\(?)'((.+)\.)?(("(?P<sequence>.+)")|(?P<sequence2>.+))'(::text\))?::(regclass|REGCLASS)\)"#,
//         )
//         .expect("compile autoincrement regex")
//     });

//     AUTOINCREMENT_REGEX.captures(value).and_then(|captures| {
//         let sequence_name = captures.name("sequence").or_else(|| captures.name("sequence2"));

//         sequence_name.and_then(|name| {
//             sequences
//                 .iter()
//                 .find(|seq| seq.name == name.as_str())
//                 .map(|seq| seq.name.as_str())
//         })
//     })
// }

/// Returns the unescaped string, as well as the remaining input after the string expression.
fn parse_string_literal(input: &str) -> Option<(String, &str)> {
    if input.len() < 2 {
        return None;
    }

    let mut out = String::with_capacity(input.len() - 2);
    let mut chars = input.char_indices().peekable();

    match chars.next()?.1 {
        '\'' => (),
        'e' | 'E' => {
            if chars.peek()?.1 == '\'' {
                unreachable!("We expect postgres to convert default strings to sql-escaping. If you see this message, Prisma is making a false assumption, please report it as a bug.")
            }
        }
        _ => return None,
    }

    let closing_quote_offset = loop {
        match chars.next() {
            Some((offset, '\'')) => match chars.peek() {
                Some((_, '\'')) => {
                    chars.next(); // consume it
                    out.push('\'')
                }
                None | Some((_, _)) => break offset,
            },
            Some((_, other)) => out.push(other),
            None => return None, // missing closing quote
        }
    };

    Some((out, &input[(closing_quote_offset + 1)..]))
}

/// Returns the unescaped string, as well as the remaining input after the quoted expression (after
/// the closing double quote).
fn parse_double_quoted_value(input: &str) -> Option<(String, &str)> {
    if input.len() < 2 {
        return None;
    }

    let mut chars = input.char_indices().peekable();
    let mut out = String::new();

    match chars.next()?.1 {
        '\"' => (),
        _ => return None,
    }

    let closing_quote_offset = loop {
        match chars.next() {
            Some((offset, '"')) => break offset,
            Some((_, '\\')) => match chars.peek()? {
                c @ ((_, '"') | (_, '\\')) => {
                    out.push(c.1);
                    chars.next();
                }
                (_, other) => {
                    out.push('\\');
                    out.push(*other);
                    chars.next();
                }
            },
            Some((_, other)) => out.push(other),
            None => return None, // missing closing quote
        }
    };

    Some((out, &input[(closing_quote_offset + 1)..]))
}

fn parse_array_literal(input: &str) -> Option<(Vec<PrismaValue>, &str)> {
    let mut values = Vec::new();

    // Array literals are always inside string literals.
    let (string_literal_contents, after_list_literal) = parse_string_literal(input)?;
    let mut chars = string_literal_contents.char_indices().peekable();
    let mut current_chars: &str = &string_literal_contents;

    // Open the array literal
    match chars.next()? {
        (_, '{') => (),
        _ => return None,
    }

    loop {
        eat_spaces(&mut chars);

        // Expect the array literal to have a next element preceded by a comma, or close.
        match chars.peek()? {
            (_, '}') => {
                chars.next();
                break;
            }
            (_, ',') => {
                chars.next();
                eat_spaces(&mut chars);
            }
            _ => (),
        }

        match chars.next()? {
            // string literals
            (offset, '\'') => {
                let remaining_input = &current_chars[offset..];
                let (string_lit, after_string_literal) = parse_string_literal(remaining_input)?;
                values.push(PrismaValue::String(string_lit));
                chars = after_string_literal.char_indices().peekable();
                current_chars = after_string_literal;
            }

            // double-quoted string
            (offset, '"') => {
                let remaining_input = &current_chars[offset..];
                let (inner_value, after_string_literal) = parse_double_quoted_value(remaining_input)?;

                match parse_string_literal(&inner_value) {
                    Some((string_lit, _)) => {
                        values.push(PrismaValue::String(string_lit));
                    }
                    None => {
                        values.push(PrismaValue::Enum(inner_value));
                    }
                }

                chars = after_string_literal.char_indices().peekable();
                current_chars = after_string_literal;
            }

            // numeric literals
            (offset, other) if other == '-' || other.is_ascii_digit() => {
                let remaining_input = &current_chars[offset..];
                let (numeric, after_literal) = parse_number_literal(remaining_input)?;
                values.push(numeric);
                chars = after_literal.char_indices().peekable();
                current_chars = after_literal;
            }

            // other (function calls, unquoted identifiers)
            (_, other) => {
                // assume an identifier
                let mut value = String::new();
                value.push(other);

                loop {
                    match chars.peek()?.1 {
                        c if c.is_alphabetic() => {
                            value.push(c);
                            chars.next();
                        }
                        _ => {
                            let value = match value.as_str() {
                                "t" => PrismaValue::Boolean(true),
                                "f" => PrismaValue::Boolean(false),
                                _ => PrismaValue::Enum(value),
                            };
                            values.push(value);
                            break;
                        }
                    }
                }
            }
        }
    }

    // ignore the end of the default string

    Some((values, after_list_literal))
}

fn parse_number_literal(input: &str) -> Option<(PrismaValue, &str)> {
    let mut value = String::new();
    let mut is_float = false;
    let mut chars = input.char_indices().peekable();

    loop {
        match chars.peek() {
            dot @ Some((_, '.')) => {
                is_float = true;
                value.push(dot.unwrap().1);
                chars.next();
            }
            Some(other) if other.1 == '-' || other.1.is_ascii_digit() => {
                value.push(other.1);
                chars.next();
            }
            Some((offset, ' ')) | Some((offset, ',')) | Some((offset, '}')) => {
                let value = if is_float {
                    PrismaValue::Float(value.parse().unwrap())
                } else {
                    PrismaValue::Int(value.parse().unwrap())
                };

                return Some((value, &input[*offset..]));
            }
            None => {
                let value = if is_float {
                    PrismaValue::Float(value.parse().unwrap())
                } else {
                    PrismaValue::Int(value.parse().unwrap())
                };

                return Some((value, ""));
            }
            _ => return None,
        }
    }
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

        let out = parse_array_constructor(&mut parser, &ColumnTypeFamily::Enum(String::new())).unwrap();

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

    // #[test]
    // fn postgres_is_sequence_works() {
    //     let sequences = vec![
    //         Sequence {
    //             name: "first_sequence".to_string(),
    //             ..Default::default()
    //         },
    //         Sequence {
    //             name: "second_sequence".to_string(),
    //             ..Default::default()
    //         },
    //         Sequence {
    //             name: "third_Sequence".to_string(),
    //             ..Default::default()
    //         },
    //         Sequence {
    //             name: "fourth_Sequence".to_string(),
    //             ..Default::default()
    //         },
    //         Sequence {
    //             name: "fifth_sequence".to_string(),
    //             ..Default::default()
    //         },
    //     ];

    //     let first_autoincrement = r#"nextval('first_sequence'::regclass)"#;
    //     assert!(is_sequence(first_autoincrement, &sequences).is_some());

    //     let second_autoincrement = r#"nextval('schema_name.second_sequence'::regclass)"#;
    //     assert!(is_sequence(second_autoincrement, &sequences).is_some());

    //     let third_autoincrement = r#"nextval('"third_Sequence"'::regclass)"#;
    //     assert!(is_sequence(third_autoincrement, &sequences).is_some());

    //     let fourth_autoincrement = r#"nextval('"schema_Name"."fourth_Sequence"'::regclass)"#;
    //     assert!(is_sequence(fourth_autoincrement, &sequences).is_some());

    //     let fifth_autoincrement = r#"nextval(('fifth_sequence'::text)::regclass)"#;
    //     assert!(is_sequence(fifth_autoincrement, &sequences).is_some());

    //     let non_autoincrement = r#"string_default_named_seq"#;
    //     assert!(is_sequence(non_autoincrement, &sequences).is_none());
    // }
}
