mod read;
mod write;

pub use read::*;
pub use write::*;

use connector::filter::RecordFinder;

#[derive(Debug, Clone)]
pub enum Query {
    Read(ReadQuery),
    Write(WriteQuery),
}

pub trait RecordFinderInjector {
    fn inject_record_finder(&mut self, rf: RecordFinder);
}

impl RecordFinderInjector for Query {
    fn inject_record_finder(&mut self, rf: RecordFinder) {
        match self {
            Self::Read(ref mut rq) => rq.inject_record_finder(rf),
            Self::Write(ref mut wq) => wq.inject_record_finder(rf),
        }
    }
}

impl std::fmt::Display for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Read(q) => write!(f, "{}", q),
            Self::Write(q) => write!(f, "{}", q),
        }
    }
}
