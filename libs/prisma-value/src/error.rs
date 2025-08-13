use alloc::borrow::Cow;
use core::{error::Error, fmt::Display};

#[derive(Debug)]
pub struct ConversionFailure {
    pub from: Cow<'static, str>,
    pub to: Cow<'static, str>,
}

impl Error for ConversionFailure {}

impl Display for ConversionFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
