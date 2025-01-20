use std::borrow::Cow;

use super::{Expression, ExpressionKind};

#[derive(Debug, Clone, PartialEq)]
pub struct Decorated<'a> {
    pub(crate) expr: Box<Expression<'a>>,
    pub(crate) prefix: Option<Cow<'a, str>>,
    pub(crate) suffix: Option<Cow<'a, str>>,
}

impl<'a> Decorated<'a> {
    pub fn new<L, R>(expr: Expression<'a>, prefix: Option<L>, suffix: Option<R>) -> Self
    where
        L: Into<Cow<'a, str>>,
        R: Into<Cow<'a, str>>,
    {
        Decorated {
            expr: Box::new(expr),
            prefix: prefix.map(<_>::into),
            suffix: suffix.map(<_>::into),
        }
    }
}

expression!(Decorated, Decorated);

pub trait Decoratable<'a> {
    fn decorate<L, R>(self, left: Option<L>, right: Option<R>) -> Decorated<'a>
    where
        L: Into<Cow<'a, str>>,
        R: Into<Cow<'a, str>>;
}

impl<'a, T> Decoratable<'a> for T
where
    T: Into<Expression<'a>>,
{
    fn decorate<L, R>(self, left: Option<L>, right: Option<R>) -> Decorated<'a>
    where
        L: Into<Cow<'a, str>>,
        R: Into<Cow<'a, str>>,
    {
        Decorated::new(self.into(), left, right)
    }
}
