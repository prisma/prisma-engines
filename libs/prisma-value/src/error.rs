use std::error::Error;
use std::fmt::Display;

#[derive(Debug)]
pub struct ConversionFailure {
    pub from: &'static str,
    pub to: &'static str,
}

impl Error for ConversionFailure {}

impl Display for ConversionFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Could not convert from `{}` to `{}`", self.from, self.to)
    }
}

impl ConversionFailure {
    pub fn new(from: &'static str, to: &'static str) -> ConversionFailure {
        ConversionFailure { from, to }
    }
}
