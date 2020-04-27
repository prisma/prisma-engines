mod aggregate_to_string;
mod average;
mod count;
mod row_number;
mod sum;

pub use aggregate_to_string::*;
pub use average::*;
pub use count::*;
pub use row_number::*;
pub use sum::*;

use super::{Aliasable, Expression};
use std::borrow::Cow;

/// A database function definition
#[derive(Debug, Clone, PartialEq)]
pub struct Function<'a> {
    pub(crate) typ_: FunctionType<'a>,
    pub(crate) alias: Option<Cow<'a, str>>,
}

/// A database function type
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FunctionType<'a> {
    RowNumber(RowNumber<'a>),
    Count(Count<'a>),
    AggregateToString(AggregateToString<'a>),
    Average(Average<'a>),
    Sum(Sum<'a>),
}

impl<'a> Aliasable<'a> for Function<'a> {
    type Target = Function<'a>;

    fn alias<T>(mut self, alias: T) -> Self::Target
    where
        T: Into<Cow<'a, str>>,
    {
        self.alias = Some(alias.into());
        self
    }
}

macro_rules! function {
    ($($kind:ident),*) => (
        $(
            impl<'a> From<$kind<'a>> for Function<'a> {
                fn from(f: $kind<'a>) -> Self {
                    Function {
                        typ_: FunctionType::$kind(f),
                        alias: None,
                    }
                }
            }

            impl<'a> From<$kind<'a>> for Expression<'a> {
                fn from(f: $kind<'a>) -> Self {
                    Function::from(f).into()
                }
            }
        )*
    );
}

function!(RowNumber, Count, AggregateToString, Average, Sum);
