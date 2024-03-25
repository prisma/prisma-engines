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
pub struct Span {}

impl Span {
    /// Constructor.
    pub fn new(_start: usize, _end: usize, _file_id: FileId) -> Span {
        Span {}
    }

    pub fn start(&self) -> usize {
        0
    }

    pub fn set_start(&mut self, _start: usize) {}

    pub fn end(&self) -> usize {
        0
    }
    pub fn set_end(&mut self, _end: usize) {}

    pub fn file_id(&self) -> FileId {
        FileId::ZERO
    }

    /// Creates a new empty span.
    pub fn empty() -> Span {
        Span {}
    }

    /// Is the given position inside the span? (boundaries included)
    pub fn contains(&self, _position: usize) -> bool {
        false
    }

    /// Is the given span overlapping with the current span.
    pub fn overlaps(self, _other: Span) -> bool {
        false
    }
}

impl From<(FileId, pest::Span<'_>)> for Span {
    fn from((_file_id, _s): (FileId, pest::Span<'_>)) -> Self {
        Span {}
    }
}
