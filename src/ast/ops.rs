use crate::ast::DatabaseValue;
use std::ops::{Add, Sub, Mul, Div};

/// Calculation operations in SQL queries.
#[derive(Debug, PartialEq, Clone)]
pub enum SqlOp<'a> {
    Add(DatabaseValue<'a>, DatabaseValue<'a>),
    Sub(DatabaseValue<'a>, DatabaseValue<'a>),
    Mul(DatabaseValue<'a>, DatabaseValue<'a>),
    Div(DatabaseValue<'a>, DatabaseValue<'a>),
}

impl<'a> Add for DatabaseValue<'a> {
    type Output = DatabaseValue<'a>;

    fn add(self, other: Self) -> Self {
        SqlOp::Add(self, other).into()
    }
}

impl<'a> Sub for DatabaseValue<'a> {
    type Output = DatabaseValue<'a>;

    fn sub(self, other: Self) -> Self {
        SqlOp::Sub(self, other).into()
    }
}

impl<'a> Mul for DatabaseValue<'a> {
    type Output = DatabaseValue<'a>;

    fn mul(self, other: Self) -> Self {
        SqlOp::Mul(self, other).into()
    }
}

impl<'a> Div for DatabaseValue<'a> {
    type Output = DatabaseValue<'a>;

    fn div(self, other: Self) -> Self {
        SqlOp::Div(self, other).into()
    }
}
