use super::{ResultRow, ResultRowRef};
use crate::ast::Value;
use std::ops;

pub trait ValueIndex<RowType, ReturnValue>: private::Sealed {
    #[doc(hidden)]
    fn index_into(self, row: &RowType) -> &ReturnValue;
}

mod private {
    pub trait Sealed {}
    impl Sealed for usize {}
    impl Sealed for &str {}
}

impl ValueIndex<ResultRowRef<'_>, Value<'static>> for usize {
    fn index_into<'v>(self, row: &'v ResultRowRef) -> &'v Value<'static> {
        row.at(self).unwrap()
    }
}

impl ValueIndex<ResultRowRef<'_>, Value<'static>> for &str {
    fn index_into<'v>(self, row: &'v ResultRowRef) -> &'v Value<'static> {
        row.get(self).unwrap()
    }
}

impl ValueIndex<ResultRow, Value<'static>> for usize {
    fn index_into(self, row: &ResultRow) -> &Value<'static> {
        row.at(self).unwrap()
    }
}

impl ValueIndex<ResultRow, Value<'static>> for &str {
    fn index_into(self, row: &ResultRow) -> &Value<'static> {
        row.get(self).unwrap()
    }
}

impl<'a, I: ValueIndex<ResultRowRef<'a>, Value<'static>> + 'static> ops::Index<I> for ResultRowRef<'a> {
    type Output = Value<'static>;

    fn index(&self, index: I) -> &Value<'static> {
        index.index_into(self)
    }
}

impl<I: ValueIndex<ResultRow, Value<'static>> + 'static> ops::Index<I> for ResultRow {
    type Output = Value<'static>;

    fn index(&self, index: I) -> &Value<'static> {
        index.index_into(self)
    }
}
