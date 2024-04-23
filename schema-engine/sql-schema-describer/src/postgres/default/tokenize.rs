#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub(super) enum Token {
    Comma,
    OpeningBrace,           // (
    ClosingBrace,           // )
    OpeningSquareBracket,   // [
    ClosingSquareBracket,   // ]
    OpeningCurly,           // {
    ClosingCurly,           // }
    Minus,                  // -
    Dot,                    // .
    CastOperator,           // ::
    DoubleQuotedIdentifier, // "
    StringLiteral,          // '...'
    CStyleStringLiteral,    // E'...'
    Digits,                 // sequence of digits without dots or sign
    Identifier,
    UnterminatedStringLiteral,
    Whitespace,
    Unknown,
}

pub(super) fn tokenize(default_string: &str) -> Vec<(Token, u32)> {
    let mut char_indices = default_string.char_indices().map(|(idx, c)| (idx as u32, c)).peekable();
    let mut out = Vec::new();

    loop {
        match char_indices.next() {
            None => return out,
            Some((start, ',')) => out.push((Token::Comma, start)),
            Some((start, '(')) => out.push((Token::OpeningBrace, start)),
            Some((start, ')')) => out.push((Token::ClosingBrace, start)),
            Some((start, '[')) => out.push((Token::OpeningSquareBracket, start)),
            Some((start, ']')) => out.push((Token::ClosingSquareBracket, start)),
            Some((start, '-')) => out.push((Token::Minus, start)),
            Some((start, '.')) => out.push((Token::Dot, start)),
            Some((start, '{')) => out.push((Token::OpeningCurly, start)),
            Some((start, '}')) => out.push((Token::ClosingCurly, start)),
            Some((start, ':')) => match char_indices.peek() {
                Some((_, ':')) => {
                    char_indices.next();
                    out.push((Token::CastOperator, start))
                }
                None | Some(_) => {
                    out.push((Token::Unknown, start));
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
                            Some((_, c)) if c.is_ascii_alphanumeric() || *c == '_' || *c == '-' => {
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
            Some((start, '"')) => loop {
                match char_indices.next() {
                    None => {
                        out.push((Token::UnterminatedStringLiteral, start));
                        return out;
                    }
                    Some((_, '"')) => {
                        out.push((Token::DoubleQuotedIdentifier, start));
                        break;
                    }
                    Some((_, '\\')) => {
                        // Consume the next character.
                        char_indices.next();
                    }
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
                    Some((_idx, c)) if c.is_ascii_alphanumeric() || *c == '_' || *c == '-' => {
                        char_indices.next();
                    }
                    None | Some(_) => {
                        out.push((Token::Identifier, start));
                        break;
                    }
                }
            },
            Some((start, _)) => out.push((Token::Unknown, start)),
        }
    }
}
