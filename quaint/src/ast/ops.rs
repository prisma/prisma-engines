use crate::ast::Expression;
use std::ops::{Add, Div, Mul, Rem, Sub};

/// Calculation operations in SQL queries.
#[derive(Debug, PartialEq, Clone)]
pub enum SqlOp<'a> {
    Add(Expression<'a>, Expression<'a>),
    Sub(Expression<'a>, Expression<'a>),
    Mul(Expression<'a>, Expression<'a>),
    Div(Expression<'a>, Expression<'a>),
    Rem(Expression<'a>, Expression<'a>),
}

impl<'a> Add for Expression<'a> {
    type Output = Expression<'a>;

    fn add(self, other: Self) -> Self {
        SqlOp::Add(self, other).into()
    }
}

impl<'a> Sub for Expression<'a> {
    type Output = Expression<'a>;

    fn sub(self, other: Self) -> Self {
        SqlOp::Sub(self, other).into()
    }
}

impl<'a> Mul for Expression<'a> {
    type Output = Expression<'a>;

    fn mul(self, other: Self) -> Self {
        SqlOp::Mul(self, other).into()
    }
}

impl<'a> Div for Expression<'a> {
    type Output = Expression<'a>;

    fn div(self, other: Self) -> Self {
        SqlOp::Div(self, other).into()
    }
}

impl<'a> Rem for Expression<'a> {
    type Output = Expression<'a>;

    fn rem(self, other: Self) -> Self {
        SqlOp::Rem(self, other).into()
    }
}
