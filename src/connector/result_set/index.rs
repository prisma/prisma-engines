use super::{ResultRow, ResultRowRef};
use crate::ast::ParameterizedValue;
use std::ops;

pub trait ValueIndex<RowType>: private::Sealed {
    #[doc(hidden)]
    fn index_into<'a>(self, row: &'a RowType) -> &'a ParameterizedValue<'static>;
}

// Prevent users from implementing the ValueIndex trait.
mod private {
    pub trait Sealed {}
    impl Sealed for usize {}
    impl Sealed for &str {}
}

impl ValueIndex<ResultRowRef<'_>> for usize {
    fn index_into<'v>(self, row: &'v ResultRowRef) -> &'v ParameterizedValue<'static> {
        row.at(self).unwrap()
    }
}

impl ValueIndex<ResultRowRef<'_>> for &str {
    fn index_into<'v>(self, row: &'v ResultRowRef) -> &'v ParameterizedValue<'static> {
        row.get(self).unwrap()
    }
}

impl ValueIndex<ResultRow> for usize {
    fn index_into<'v>(self, row: &'v ResultRow) -> &'v ParameterizedValue<'static> {
        row.at(self).unwrap()
    }
}

impl ValueIndex<ResultRow> for &str {
    fn index_into<'v>(self, row: &'v ResultRow) -> &'v ParameterizedValue<'static> {
        row.get(self).unwrap()
    }
}

impl<'a, I: ValueIndex<ResultRowRef<'a>> + 'static> ops::Index<I> for ResultRowRef<'a> {
    type Output = ParameterizedValue<'static>;

    fn index(&self, index: I) -> &ParameterizedValue<'static> {
        index.index_into(self)
    }
}

impl<I: ValueIndex<ResultRow> + 'static> ops::Index<I> for ResultRow {
    type Output = ParameterizedValue<'static>;

    fn index(&self, index: I) -> &ParameterizedValue<'static> {
        index.index_into(self)
    }
}
