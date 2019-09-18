use super::Path;
use connector::filter::RecordFinder;

pub struct RecordAddress {
    pub path: Path,
    pub record_finder: RecordFinder,
}
