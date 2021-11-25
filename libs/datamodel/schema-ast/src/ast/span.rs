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
    pub(crate) fn from_pest(s: pest::Span<'_>) -> Span {
        Span {
            start: s.start(),
            end: s.end(),
        }
    }

    /// Adds an offset to a span.
    pub fn lift_span(&self, offset: usize) -> Span {
        Span {
            start: offset + self.start,
            end: offset + self.end,
        }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{} - {}]", self.start, self.end)
    }
}

impl From<pest::Span<'_>> for Span {
    fn from(s: pest::Span<'_>) -> Self {
        Span {
            start: s.start(),
            end: s.end(),
        }
    }
}
