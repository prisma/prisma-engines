mod count;
mod distinct;
mod row_number;

pub use count::*;
pub use distinct::*;
pub use row_number::*;

use super::DatabaseValue;

/// A database function definition
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub(crate) typ_: FunctionType,
    pub(crate) alias: Option<String>,
}

/// A database function type
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FunctionType {
    RowNumber(RowNumber),
    Count(Count),
    Distinct(Distinct),
}

impl Function {
    /// Give the function an alias in the query.
    pub fn alias<S>(mut self, alias: S) -> Self
    where
        S: Into<String>,
    {
        self.alias = Some(alias.into());
        self
    }
}

macro_rules! function {
    ($($kind:ident),*) => (
        $(
            impl From<$kind> for Function {
                #[inline]
                fn from(f: $kind) -> Function {
                    Function {
                        typ_: FunctionType::$kind(f),
                        alias: None,
                    }
                }
            }

            impl From<$kind> for DatabaseValue {
                #[inline]
                fn from(f: $kind) -> DatabaseValue {
                    Function::from(f).into()
                }
            }
        )*
    );
}

function!(RowNumber, Distinct, Count);
