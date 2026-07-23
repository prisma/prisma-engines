use lsp_types::{Position, Range};
use psl::{diagnostics::FileId, parser_database::ast::Span};

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
pub(crate) fn range_to_span(range: Range, document: &str, file_id: FileId) -> Span {
    let start = position_to_offset(&range.start, document).unwrap();
    let end = position_to_offset(&range.end, document).unwrap();

    Span::new(start, end, file_id)
}

/// Gives the LSP position right after the given span, skipping any trailing newlines
pub(crate) fn position_after_span(span: Span, document: &str) -> Position {
    let end = match (document.chars().nth(span.end - 2), document.chars().nth(span.end - 1)) {
        (Some('\r'), Some('\n')) => span.end - 2,
        (_, Some('\n')) => span.end - 1,
        _ => span.end,
    };

    offset_to_position(end, document)
}

fn offset_to_position_and_next_offset(
    offset: usize,
    document: &str,
    initial_position: Position,
    initial_lsp_offset: usize,
    initial_offset: usize,
) -> (Position, usize, usize) {
    let mut position = initial_position;
    let mut current_lsp_offset = initial_lsp_offset;
    let mut current_offset = initial_offset;

    for chr in document[current_offset..].chars() {
        match chr {
            _ if offset <= current_offset => {
                return (position, current_lsp_offset, current_offset);
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
        current_lsp_offset += chr.len_utf16();
    }

    (position, current_lsp_offset, current_offset)
}

#[allow(dead_code)]
/// Converts the byte offset to the offset used in the LSP, which is the number of the UTF-16 code unit.
pub(crate) fn offset_to_lsp_offset(offset: usize, document: &str) -> usize {
    let (_, lsp_offset, _) = offset_to_position_and_next_offset(offset, document, Position::default(), 0, 0);
    lsp_offset
}

/// Converts a byte offset to an LSP position, if the given offset
/// does not overflow the document.
fn offset_to_position(offset: usize, document: &str) -> Position {
    let (position, _, _) = offset_to_position_and_next_offset(offset, document, Position::default(), 0, 0);
    position
}

fn span_to_range_and_lsp_offsets(span: Span, document: &str) -> (Range, (usize, usize)) {
    let (start_position, start_lsp_offset, next_offset) =
        offset_to_position_and_next_offset(span.start, document, Position::default(), 0, 0);
    let (end_position, end_lsp_offset, _) =
        offset_to_position_and_next_offset(span.end, document, start_position, start_lsp_offset, next_offset);

    (
        Range::new(start_position, end_position),
        (start_lsp_offset, end_lsp_offset),
    )
}

/// Converts a span to a pair of LSP offsets.
pub fn span_to_lsp_offsets(span: Span, document: &str) -> (usize, usize) {
    let (_, lsp_offsets) = span_to_range_and_lsp_offsets(span, document);

    lsp_offsets
}

/// Converts a span to a pair of LSP positions.
pub fn span_to_range(span: Span, document: &str) -> Range {
    let (range, _) = span_to_range_and_lsp_offsets(span, document);

    range
}

#[cfg(test)]
mod tests {
    use lsp_types::{Position, Range};
    use psl::diagnostics::{FileId, Span};

    // On Windows, a newline is actually two characters.
    #[test]
    fn position_to_offset_with_crlf() {
        let schema = "\r\nmodel Test {\r\n    id Int @id\r\n}";
        // Let's put the cursor on the "i" in "id Int".
        let expected_offset = schema.bytes().position(|c| c == b'i').unwrap();
        let found_offset = super::position_to_offset(&Position { line: 2, character: 4 }, schema).unwrap();

        assert_eq!(found_offset, expected_offset);
    }

    #[test]
    fn position_after_span_no_newline() {
        let str = "some string";
        let span = Span::new(0, str.len(), FileId::ZERO);
        let pos = super::position_after_span(span, str);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 11);
    }

    #[test]
    fn position_after_span_lf() {
        let str = "some string\n";
        let span = Span::new(0, str.len(), FileId::ZERO);
        let pos = super::position_after_span(span, str);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 11);
    }

    #[test]
    fn position_after_span_crlf() {
        let str = "some string\r\n";
        let span = Span::new(0, str.len(), FileId::ZERO);
        let pos = super::position_after_span(span, str);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 11);
    }

    // In the LSP protocol, the number of the UTF-16 code unit should be used as the offset.
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

    #[test]
    fn span_to_range_with_multibyte() {
        let schema = "// 🌐 ｍｕｌｔｉｂｙｔｅ\n^😀$\n";

        let span = Span::new(
            schema.bytes().position(|c| c == b'^').unwrap(),
            schema.bytes().position(|c| c == b'$').unwrap(),
            FileId::ZERO,
        );
        let expected_range = Range::new(Position { line: 1, character: 0 }, Position { line: 1, character: 3 });
        let found_range = super::span_to_range(span, schema);

        assert_eq!(expected_range, found_range);
    }
}
