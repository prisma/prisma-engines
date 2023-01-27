use std::borrow::Cow;
use std::error::Error;
use std::fmt::Display;

#[derive(Debug)]
pub struct ConversionFailure {
    pub from: Cow<'static, str>,
    pub to: Cow<'static, str>,
}

impl Error for ConversionFailure {}

impl Display for ConversionFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Could not convert from `{}` to `{}`", self.from, self.to)
    }
}

impl ConversionFailure {
    pub fn new<A, B>(from: A, to: B) -> ConversionFailure
    where
        A: Into<Cow<'static, str>>,
        B: Into<Cow<'static, str>>,
    {
        ConversionFailure {
            from: from.into(),
            to: to.into(),
        }
    }
}
