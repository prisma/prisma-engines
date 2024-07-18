/// The stable identifier for a PSL file.
#[derive(Debug, PartialEq, Clone, Copy, Hash, Eq, PartialOrd, Ord)]
pub struct FileId(pub u32); // we can't encapsulate because it would be a circular crate
                            // dependency between diagnostics and parser-database

impl FileId {
    pub const ZERO: FileId = FileId(0);
    pub const MAX: FileId = FileId(u32::MAX);
}

/// Represents a location in a datamodel's text representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub file_id: FileId,
}

impl Span {
    /// Constructor.
    pub fn new(start: usize, end: usize, file_id: FileId) -> Span {
        Span { start, end, file_id }
    }

    /// Creates a new empty span.
    pub fn empty() -> Span {
        Span {
            start: 0,
            end: 0,
            file_id: FileId::ZERO,
        }
    }

    /// Is the given position inside the span? (boundaries included)
    pub fn contains(&self, position: usize) -> bool {
        position >= self.start && position <= self.end
    }

    /// Is the given span overlapping with the current span.
    pub fn overlaps(self, other: Span) -> bool {
        self.file_id == other.file_id && (self.contains(other.start) || self.contains(other.end))
    }
}

impl From<(FileId, pest::Span<'_>)> for Span {
    fn from((file_id, s): (FileId, pest::Span<'_>)) -> Self {
        Span {
            start: s.start(),
            end: s.end(),
            file_id,
        }
    }
}
