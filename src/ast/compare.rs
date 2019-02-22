use crate::ast::{And, Column, ConditionTree, DatabaseValue, Expression, Row};

#[derive(Debug, Clone, PartialEq)]
pub enum Compare {
    Equals(Box<DatabaseValue>, Box<DatabaseValue>),
    NotEquals(Box<DatabaseValue>, Box<DatabaseValue>),
    LessThan(Box<DatabaseValue>, Box<DatabaseValue>),
    LessThanOrEquals(Box<DatabaseValue>, Box<DatabaseValue>),
    GreaterThan(Box<DatabaseValue>, Box<DatabaseValue>),
    GreaterThanOrEquals(Box<DatabaseValue>, Box<DatabaseValue>),
    In(Box<DatabaseValue>, Box<Row>),
    NotIn(Box<DatabaseValue>, Box<Row>),
    Null(Box<DatabaseValue>),
    NotNull(Box<DatabaseValue>),
}

impl Into<ConditionTree> for Compare {
    fn into(self) -> ConditionTree {
        let expression: Expression = self.into();
        ConditionTree::single(expression)
    }
}

impl Into<Expression> for Compare {
    fn into(self) -> Expression {
        Expression::Compare(self)
    }
}

impl And for Compare {
    fn and<E>(self, other: E) -> ConditionTree
    where
        E: Into<Expression>,
    {
        let left: Expression = self.into();
        let right: Expression = other.into();

        ConditionTree::and(left, right)
    }
}

pub trait Comparable {
    fn equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>;

    fn not_equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>;

    fn less_than<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>;

    fn less_than_or_equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>;

    fn greater_than<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>;

    fn greater_than_or_equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>;

    fn in_selection<T>(self, selection: Vec<T>) -> Compare
    where
        T: Into<DatabaseValue>;

    fn not_in_selection<T>(self, selection: Vec<T>) -> Compare
    where
        T: Into<DatabaseValue>;

    fn is_null(self) -> Compare;
    fn is_not_null(self) -> Compare;
}

impl Comparable for DatabaseValue {
    #[inline]
    fn equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        Compare::Equals(Box::new(self), Box::new(comparison.into()))
    }

    #[inline]
    fn not_equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        Compare::NotEquals(Box::new(self), Box::new(comparison.into()))
    }

    #[inline]
    fn less_than<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        Compare::LessThan(Box::new(self), Box::new(comparison.into()))
    }

    #[inline]
    fn less_than_or_equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        Compare::LessThanOrEquals(Box::new(self), Box::new(comparison.into()))
    }

    #[inline]
    fn greater_than<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        Compare::GreaterThan(Box::new(self), Box::new(comparison.into()))
    }

    #[inline]
    fn greater_than_or_equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        Compare::GreaterThanOrEquals(Box::new(self), Box::new(comparison.into()))
    }

    #[inline]
    fn in_selection<T>(self, selection: Vec<T>) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        Compare::In(Box::new(self), Box::new(Row::from(selection).into()))
    }

    #[inline]
    fn not_in_selection<T>(self, selection: Vec<T>) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        Compare::NotIn(Box::new(self), Box::new(Row::from(selection).into()))
    }

    #[inline]
    fn is_null(self) -> Compare {
        Compare::Null(Box::new(self))
    }

    #[inline]
    fn is_not_null(self) -> Compare {
        Compare::NotNull(Box::new(self))
    }
}

#[macro_export]
macro_rules! comparable {
    ($($kind:ty),*) => (
        $(
            impl Comparable for $kind {
                #[inline]
                fn equals<T>(self, comparison: T) -> Compare
                where
                    T: Into<DatabaseValue>,
                {
                    let col: Column = self.into();
                    let val: DatabaseValue = col.into();
                    val.equals(comparison)
                }

                #[inline]
                fn not_equals<T>(self, comparison: T) -> Compare
                where
                    T: Into<DatabaseValue>,
                {
                    let col: Column = self.into();
                    let val: DatabaseValue = col.into();
                    val.not_equals(comparison)
                }

                #[inline]
                fn less_than<T>(self, comparison: T) -> Compare
                where
                    T: Into<DatabaseValue>,
                {
                    let col: Column = self.into();
                    let val: DatabaseValue = col.into();
                    val.less_than(comparison)
                }

                #[inline]
                fn less_than_or_equals<T>(self, comparison: T) -> Compare
                where
                    T: Into<DatabaseValue>,
                {
                    let col: Column = self.into();
                    let val: DatabaseValue = col.into();
                    val.less_than_or_equals(comparison)
                }

                #[inline]
                fn greater_than<T>(self, comparison: T) -> Compare
                where
                    T: Into<DatabaseValue>,
                {
                    let col: Column = self.into();
                    let val: DatabaseValue = col.into();
                    val.greater_than(comparison)
                }

                #[inline]
                fn greater_than_or_equals<T>(self, comparison: T) -> Compare
                where
                    T: Into<DatabaseValue>,
                {
                    let col: Column = self.into();
                    let val: DatabaseValue = col.into();
                    val.greater_than_or_equals(comparison)
                }

                #[inline]
                fn in_selection<T>(self, selection: Vec<T>) -> Compare
                where
                    T: Into<DatabaseValue>,
                {
                    let col: Column = self.into();
                    let val: DatabaseValue = col.into();
                    val.in_selection(selection)
                }

                #[inline]
                fn not_in_selection<T>(self, selection: Vec<T>) -> Compare
                where
                    T: Into<DatabaseValue>,
                {
                    let col: Column = self.into();
                    let val: DatabaseValue = col.into();
                    val.not_in_selection(selection)
                }

                #[inline]
                fn is_null(self) -> Compare {
                    let col: Column = self.into();
                    let val: DatabaseValue = col.into();
                    val.is_null()
                }

                #[inline]
                fn is_not_null(self) -> Compare {
                    let col: Column = self.into();
                    let val: DatabaseValue = col.into();
                    val.is_not_null()
                }
            }
        )*
    );
}

comparable!(&str, (&str, &str, &str), (&str, &str));
