mod read;
mod write;

pub use read::*;
pub use write::*;

#[derive(Debug, Clone)]
pub enum Query {
    Read(ReadQuery),
    Write(WriteQuery),
}

impl std::fmt::Display for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Read(q) => write!(f, "{}", q),
            Self::Write(q) => write!(f, "{}", q),
        }
    }
}
