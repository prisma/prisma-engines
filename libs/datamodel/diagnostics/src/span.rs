/// Represents a location in a datamodel's text representation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    /// Constructor.
    pub fn new(start: usize, end: usize) -> Span {
        Span { start, end }
    }

    /// Creates a new empty span.
    pub fn empty() -> Span {
        Span { start: 0, end: 0 }
    }

    /// Is the given position inside the span? (boundaries included)
    pub fn contains(&self, position: usize) -> bool {
        position >= self.start && position <= self.end
    }

    /// Is the given span overlapping with the current span.
    pub fn overlaps(self, other: Span) -> bool {
        self.contains(other.start) || self.contains(other.end)
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
