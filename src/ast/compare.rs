use crate::ast::{Column, DatabaseValue, ParameterizedValue, Row};

#[derive(Debug, Clone, PartialEq)]
pub enum Compare {
    Equals(Box<DatabaseValue>, Box<DatabaseValue>),
    NotEquals(Box<DatabaseValue>, Box<DatabaseValue>),
    LessThan(Box<DatabaseValue>, Box<DatabaseValue>),
    LessThanOrEquals(Box<DatabaseValue>, Box<DatabaseValue>),
    GreaterThan(Box<DatabaseValue>, Box<DatabaseValue>),
    GreaterThanOrEquals(Box<DatabaseValue>, Box<DatabaseValue>),
    In(Box<DatabaseValue>, Box<DatabaseValue>),
    NotIn(Box<DatabaseValue>, Box<DatabaseValue>),
    Null(Box<DatabaseValue>),
    NotNull(Box<DatabaseValue>),
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
        T: Into<ParameterizedValue>;

    fn not_in_selection<T>(self, selection: Vec<T>) -> Compare
    where
        T: Into<ParameterizedValue>;

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
        T: Into<ParameterizedValue>,
    {
        Compare::In(Box::new(self), Box::new(Row::from(selection).into()))
    }

    #[inline]
    fn not_in_selection<T>(self, selection: Vec<T>) -> Compare
    where
        T: Into<ParameterizedValue>,
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
                    let val: DatabaseValue = self.into();
                    val.equals(comparison)
                }

                #[inline]
                fn not_equals<T>(self, comparison: T) -> Compare
                where
                    T: Into<DatabaseValue>,
                {
                    let val: DatabaseValue = self.into();
                    val.not_equals(comparison)
                }

                #[inline]
                fn less_than<T>(self, comparison: T) -> Compare
                where
                    T: Into<DatabaseValue>,
                {
                    let val: DatabaseValue = self.into();
                    val.less_than(comparison)
                }

                #[inline]
                fn less_than_or_equals<T>(self, comparison: T) -> Compare
                where
                    T: Into<DatabaseValue>,
                {
                    let val: DatabaseValue = self.into();
                    val.less_than_or_equals(comparison)
                }

                #[inline]
                fn greater_than<T>(self, comparison: T) -> Compare
                where
                    T: Into<DatabaseValue>,
                {
                    let val: DatabaseValue = self.into();
                    val.greater_than(comparison)
                }

                #[inline]
                fn greater_than_or_equals<T>(self, comparison: T) -> Compare
                where
                    T: Into<DatabaseValue>,
                {
                    let val: DatabaseValue = self.into();
                    val.greater_than_or_equals(comparison)
                }

                #[inline]
                fn in_selection<T>(self, selection: Vec<T>) -> Compare
                where
                    T: Into<ParameterizedValue>,
                {
                    let val: DatabaseValue = self.into();
                    val.in_selection(selection)
                }

                #[inline]
                fn not_in_selection<T>(self, selection: Vec<T>) -> Compare
                where
                    T: Into<ParameterizedValue>,
                {
                    let val: DatabaseValue = self.into();
                    val.not_in_selection(selection)
                }

                #[inline]
                fn is_null(self) -> Compare {
                    let val: DatabaseValue = self.into();
                    val.is_null()
                }

                #[inline]
                fn is_not_null(self) -> Compare {
                    let val: DatabaseValue = self.into();
                    val.is_not_null()
                }
            }
        )*
    );
}

comparable!(&str, Row, Column, String, i64, f64, bool);
