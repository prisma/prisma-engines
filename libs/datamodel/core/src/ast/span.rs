/// Represents a location in a datamodel's text representation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Span {
        Span { start, end }
    }

    // Creates a new empty span.
    pub fn empty() -> Span {
        Span { start: 0, end: 0 }
    }
    /// Creates a new ast::Span from a pest::Span.
    pub fn from_pest(s: pest::Span) -> Span {
        Span {
            start: s.start(),
            end: s.end(),
        }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "[{} - {}]", self.start, self.end)
    }
}

/// Adds an offset to a span.
pub fn lift_span(span: &Span, offset: usize) -> Span {
    Span {
        start: offset + span.start,
        end: offset + span.end,
    }
}
