use lsp_types::{Position, Range};
use psl::parser_database::ast::Span;

/// The LSP position is expressed as a (line, col) tuple, but our pest-based parser works with byte
/// offsets. This function converts from an LSP position to a pest byte offset. Returns `None` if
/// the position has a line past the end of the document, or a character position past the end of
/// the line.
pub(crate) fn position_to_offset(position: &Position, document: &str) -> Option<usize> {
    let mut offset = 0;
    let mut line_offset = position.line;
    let mut character_offset = position.character as i64;
    let mut chars = document.chars();

    while line_offset > 0 {
        loop {
            match chars.next() {
                Some('\n') => {
                    offset += '\n'.len_utf8();
                    break;
                }
                Some(chr) => {
                    offset += chr.len_utf8();
                }
                None => return Some(offset),
            }
        }

        line_offset -= 1;
    }

    while character_offset > 0 {
        match chars.next() {
            Some('\n') | None => return Some(offset),
            Some(chr) => {
                offset += chr.len_utf8();
                character_offset -= chr.len_utf16() as i64;
            }
        }
    }

    Some(offset)
}

#[track_caller]
/// Converts an LSP range to a span.
pub(crate) fn range_to_span(range: Range, document: &str) -> Span {
    let start = position_to_offset(&range.start, document).unwrap();
    let end = position_to_offset(&range.end, document).unwrap();

    Span::new(start, end, psl::parser_database::FileId::ZERO)
}

/// Gives the LSP position right after the given span.
pub(crate) fn position_after_span(span: Span, document: &str) -> Position {
    offset_to_position(span.end - 1, document)
}

/// Converts the byte offset to the offset used in LSP
pub(crate) fn offset_to_lsp_offset(offset: usize, document: &str) -> usize {
    let mut current_offset = 0;
    let mut current_lsp_offset = 0;

    for chr in document.chars() {
        if offset <= current_offset {
            break;
        }
        current_offset += chr.len_utf8();
        current_lsp_offset += chr.len_utf16();
    }

    current_lsp_offset
}

/// Converts a byte offset to an LSP position, if the given offset
/// does not overflow the document.
pub fn offset_to_position(offset: usize, document: &str) -> Position {
    let mut current_offset = 0;
    let mut position = Position::default();

    for chr in document.chars() {
        match chr {
            _ if offset <= current_offset => {
                return position;
            }
            '\n' => {
                position.character = 0;
                position.line += 1;
            }
            _ => {
                position.character += chr.len_utf16() as u32;
            }
        }
        current_offset += chr.len_utf8();
    }

    position
}

#[cfg(test)]
mod tests {
    use lsp_types::Position;

    // On Windows, a newline is actually two characters.
    #[test]
    fn position_to_offset_with_crlf() {
        let schema = "\r\nmodel Test {\r\n    id Int @id\r\n}";
        // Let's put the cursor on the "i" in "id Int".
        let expected_offset = schema.bytes().position(|c| c == b'i').unwrap();
        let found_offset = super::position_to_offset(&Position { line: 2, character: 4 }, schema).unwrap();

        assert_eq!(found_offset, expected_offset);
    }

    // LSP server should return utf-16 offset
    #[test]
    fn offset_to_position_with_multibyte() {
        let schema = "// 🌐 ｍｕｌｔｉｂｙｔｅ\n😀@\n";

        let cursor_offset = schema.bytes().position(|c| c == b'@').unwrap();
        let expected_position = Position { line: 1, character: 2 };
        let found_position = super::offset_to_position(cursor_offset, schema);

        assert_eq!(expected_position, found_position);
    }
    #[test]
    fn position_to_offset_with_multibyte() {
        let schema = "// 🌐 ｍｕｌｔｉｂｙｔｅ\n😀@\n";

        let expected_offset = schema.bytes().position(|c| c == b'@').unwrap();
        let found_offset = super::position_to_offset(&Position { line: 1, character: 2 }, schema).unwrap();

        assert_eq!(expected_offset, found_offset);
    }
}
