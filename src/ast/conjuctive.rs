use crate::ast::{ConditionTree, Expression};

/// `AND`, `OR` and `NOT` conjuctive implementations.
pub trait Conjuctive {
    /// Builds an `AND` condition having `self` as the left leaf and `other` as the right.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// assert_eq!(
    ///     "foo".equals("bar").and("wtf".less_than(3)),
    ///     ConditionTree::and("foo".equals("bar"), "wtf".less_than(3))
    /// )
    /// ```
    fn and<E>(self, other: E) -> ConditionTree
    where
        E: Into<Expression>;

    /// Builds an `OR` condition having `self` as the left leaf and `other` as the right.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// assert_eq!(
    ///     "foo".equals("bar").or("wtf".less_than(3)),
    ///     ConditionTree::or("foo".equals("bar"), "wtf".less_than(3))
    /// )
    /// ```
    fn or<E>(self, other: E) -> ConditionTree
    where
        E: Into<Expression>;

    /// Builds a `NOT` condition having `self` as the condition.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// assert_eq!(
    ///     "foo".equals("bar").not(),
    ///     ConditionTree::not("foo".equals("bar"))
    /// )
    /// ```
    fn not(self) -> ConditionTree;
}

impl<T> Conjuctive for T where T: Into<Expression> {
    #[inline]
    fn and<E>(self, other: E) -> ConditionTree
    where
        E: Into<Expression>
    {
        ConditionTree::and(self.into(), other.into())
    }

    #[inline]
    fn or<E>(self, other: E) -> ConditionTree
    where
        E: Into<Expression>
    {
        ConditionTree::or(self.into(), other.into())
    }

    #[inline]
    fn not(self) -> ConditionTree {
        ConditionTree::not(self.into())
    }
}
