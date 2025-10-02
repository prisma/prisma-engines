//! Scalar list defaults of the form `'{}'`.
//! Reference: <https://www.postgresql.org/docs/current/arrays.html>

use super::{Parser, Token, tokenize};
use crate::{ColumnType, ColumnTypeFamily};
use prisma_value::PrismaValue;

pub(super) fn parse_array_literal(input: &str, tpe: &ColumnType) -> Option<Vec<PrismaValue>> {
    let mut values = Vec::new();
    let tokens = tokenize(input);
    let mut parser = Parser::new(input, &tokens);

    parser.expect(Token::OpeningCurly)?;

    loop {
        match parser.peek_token()? {
            Token::ClosingCurly => {
                parser.expect(Token::ClosingCurly)?;
                break;
            }
            Token::Identifier => {
                let s = parser.expect(Token::Identifier)?;
                values.push(parse_literal(s, tpe)?);
            }
            Token::DoubleQuotedIdentifier => {
                let s = parser.expect(Token::DoubleQuotedIdentifier)?;
                let unquoted = super::parse_double_quoted_string_contents(s);
                values.push(parse_literal(&unquoted, tpe)?);
            }
            Token::StringLiteral => {
                let s = super::parse_string_value(&mut parser)?;
                values.push(parse_literal(&s, tpe)?)
            }
            Token::Minus | Token::Digits => match tpe.family {
                ColumnTypeFamily::Int | ColumnTypeFamily::BigInt => {
                    let value = super::parse_int_default(&mut parser)?;
                    values.push(value.as_value().unwrap().to_owned());
                }
                ColumnTypeFamily::Decimal | ColumnTypeFamily::Float => {
                    let value = super::parse_float_default(&mut parser)?;
                    values.push(value.as_value().unwrap().to_owned());
                }
                _ => return None,
            },
            _ => return None,
        }

        match parser.peek_token()? {
            Token::Comma => {
                parser.expect(Token::Comma)?;
            }
            Token::ClosingCurly => {
                parser.expect(Token::ClosingCurly)?;
                break;
            }
            _ => return None,
        }
    }

    // ignore the end of the default string

    Some(values)
}

fn parse_literal(s: &str, tpe: &ColumnType) -> Option<PrismaValue> {
    match tpe.family {
        ColumnTypeFamily::Int | ColumnTypeFamily::BigInt => Some(PrismaValue::BigInt(s.parse().ok()?)),
        ColumnTypeFamily::Float | ColumnTypeFamily::Decimal => Some(PrismaValue::Float(s.parse().ok()?)),
        ColumnTypeFamily::String => Some(PrismaValue::String(s.to_owned())),
        ColumnTypeFamily::Boolean => match s {
            s if s.eq_ignore_ascii_case("t") || s.eq_ignore_ascii_case("true") => Some(PrismaValue::Boolean(true)),
            s if s.eq_ignore_ascii_case("f") || s.eq_ignore_ascii_case("false") || s.eq_ignore_ascii_case("false") => {
                Some(PrismaValue::Boolean(false))
            }
            _ => None,
        },
        ColumnTypeFamily::Json => Some(PrismaValue::Json(s.to_owned())),
        ColumnTypeFamily::Enum(_) => {
            let tokens = tokenize(s);
            let mut parser = Parser::new(s, &tokens);
            match super::parse_string_value(&mut parser) {
                Some(string_contents) => Some(PrismaValue::Enum(string_contents)),
                None => Some(PrismaValue::Enum(s.to_owned())),
            }
        }
        ColumnTypeFamily::DateTime
        | ColumnTypeFamily::Binary
        | ColumnTypeFamily::Uuid
        | ColumnTypeFamily::Udt(_)
        | ColumnTypeFamily::Unsupported(_) => None,
    }
}
