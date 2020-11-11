use std::{io, num};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{}", _0)]
    Io(io::Error),
    #[error("{}", _0)]
    ParseInt(num::ParseIntError),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<num::ParseIntError> for Error {
    fn from(e: num::ParseIntError) -> Self {
        Self::ParseInt(e)
    }
}
