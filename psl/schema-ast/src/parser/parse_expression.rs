use super::{
    helpers::{parsing_catch_all, Pair},
    parse_arguments::parse_arguments_list,
    Rule,
};
use crate::ast::*;
use diagnostics::{DatamodelError, Diagnostics};

pub(crate) fn parse_expression(token: Pair<'_>, diagnostics: &mut diagnostics::Diagnostics) -> Expression {
    let first_child = token.into_inner().next().unwrap();
    let span = Span::from(first_child.as_span());
    match first_child.as_rule() {
        Rule::numeric_literal => Expression::NumericValue(first_child.as_str().to_string(), span),
        Rule::string_literal => Expression::StringValue(parse_string_literal(first_child, diagnostics), span),
        Rule::path => Expression::ConstantValue(first_child.as_str().to_string(), span),
        Rule::function_call => parse_function(first_child, diagnostics),
        Rule::array_expression => parse_array(first_child, diagnostics),
        _ => unreachable!(
            "Encountered impossible literal during parsing: {:?}",
            first_child.tokens()
        ),
    }
}

fn parse_function(pair: Pair<'_>, diagnostics: &mut Diagnostics) -> Expression {
    let mut name: Option<String> = None;
    let mut arguments = ArgumentsList::default();
    let (pair_str, span) = (pair.as_str(), pair.as_span());

    for current in pair.into_inner() {
        match current.as_rule() {
            Rule::path => name = Some(current.as_str().to_string()),
            Rule::arguments_list => parse_arguments_list(current, &mut arguments, diagnostics),
            _ => parsing_catch_all(&current, "function"),
        }
    }

    match name {
        Some(name) => Expression::Function(name, arguments, Span::from(span)),
        _ => unreachable!("Encountered impossible function during parsing: {:?}", pair_str),
    }
}

fn parse_array(token: Pair<'_>, diagnostics: &mut Diagnostics) -> Expression {
    let mut elements: Vec<Expression> = vec![];
    let span = token.as_span();

    for current in token.into_inner() {
        match current.as_rule() {
            Rule::expression => elements.push(parse_expression(current, diagnostics)),
            _ => parsing_catch_all(&current, "array"),
        }
    }

    Expression::Array(elements, Span::from(span))
}

fn parse_string_literal(token: Pair<'_>, diagnostics: &mut Diagnostics) -> String {
    assert!(token.as_rule() == Rule::string_literal);
    let contents = token.clone().into_inner().next().unwrap();
    let contents_str = contents.as_str();

    // This will overallocate a bit for strings with escaped characters, but it
    // shouldn't make a dramatic difference.
    let mut out = String::with_capacity(contents_str.len());
    let mut chars = contents_str.char_indices();

    // https://datatracker.ietf.org/doc/html/rfc8259#section-7
    while let Some((start, c)) = chars.next() {
        match c {
            '\\' => match chars.next().unwrap() {
                (_, '"') => {
                    out.push('"');
                }
                (_, '\\') => {
                    out.push('\\');
                }
                (_, '/') => {
                    out.push('/');
                }
                (_, 'b') => {
                    out.push('\u{0008}');
                }
                (_, 'f') => {
                    out.push('\u{000C}');
                }
                (_, 'n') => {
                    out.push('\n');
                }
                (_, 'r') => {
                    out.push('\r');
                }
                (_, 't') => {
                    out.push('\t');
                }
                (_, 'u') => {
                    let (advance, char) = try_parse_unicode_codepoint(
                        &contents_str[start..],
                        contents.as_span().start() + start,
                        diagnostics,
                    );

                    if let Some(char) = char {
                        out.push(char);
                    }

                    for _ in 0..advance.saturating_sub(2) {
                        chars.next().unwrap();
                    }
                }
                (_, c) => {
                    let mut final_span: crate::ast::Span = contents.as_span().into();
                    final_span.start += start;
                    final_span.end = final_span.start + 1 + c.len_utf8();
                    diagnostics.push_error(DatamodelError::new_static(
                        r#"Unknown escape sequence. If the value is a windows-style path, `\` must be escaped as `\\`."#,
                        final_span
                    ));
                }
            },
            other => out.push(other),
        }
    }

    out
}

/// https://datatracker.ietf.org/doc/html/rfc8259#section-7
///
/// Returns the parsed character and how much input (in bytes) was consumed.
fn try_parse_unicode_codepoint(
    slice: &str,
    slice_offset: usize,
    diagnostics: &mut Diagnostics,
) -> (usize, Option<char>) {
    let unicode_sequence_error = |consumed| {
        let span = crate::ast::Span {
            start: slice_offset,
            end: (slice_offset + slice.len()).min(slice_offset + consumed),
        };
        DatamodelError::new_static("Invalid unicode escape sequence.", span)
    };

    match parse_codepoint(slice) {
        (consumed, None) => {
            diagnostics.push_error(unicode_sequence_error(consumed.max(2)));
            (consumed, None)
        }
        (consumed_first_codepoint, Some(first_codepoint)) => {
            // Check if the first codepoint is a valid UTF-8 codepoint. UTF-16 surrogate sequences
            // are not valid UTF-8, so we can do this safely.
            if let Some(c) = char::from_u32(first_codepoint.into()) {
                return (6, Some(c));
            }

            // If that doesn't work, try parsing a second codepoint, and treat the first one as a
            // UTF-16 surrogate pair.
            match parse_codepoint(&slice[6..]) {
                (_, None) => {
                    diagnostics.push_error(unicode_sequence_error(consumed_first_codepoint));
                    (consumed_first_codepoint, None)
                }
                (consumed_second_codepoint, Some(second_codepoint)) => {
                    // UTF-16 surrogate with
                    let char = match char::decode_utf16([first_codepoint, second_codepoint].into_iter()).next() {
                        Some(Ok(c)) => Some(c),
                        _ => {
                            diagnostics.push_error(unicode_sequence_error(
                                consumed_first_codepoint + consumed_second_codepoint,
                            ));
                            None
                        }
                    };

                    (consumed_first_codepoint * 2, char)
                }
            }
        }
    }
}

fn parse_codepoint(slice: &str) -> (usize, Option<u16>) {
    if slice.len() < 4 || !slice.starts_with("\\u") {
        return (0, None);
    }

    let mut chars = slice[2..].chars();
    let mut codepoint = 0u16;

    // four nibbles (4 bit integers)
    for i in 0u8..4 {
        let nibble_offset = 3 - i;
        match chars.next().and_then(|c| c.to_digit(16)) {
            Some(nibble) => {
                codepoint += (nibble as u16) << (nibble_offset * 4);
            }
            None => return (2 + i as usize, None),
        }
    }

    (6, Some(codepoint))
}
