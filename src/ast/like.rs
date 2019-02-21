use crate::ast::{DatabaseValue, Expression};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LikeType {
    Like,
    NotLike,
    StartsWith,
    NotStartsWith,
    EndsWith,
    NotEndsWith,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Like {
    typ: LikeType,
    expression: Expression,
    value: DatabaseValue,
}

pub trait Likable {
    fn create_like(typ: LikeType, expression: Expression, value: String) -> Like {
        Like {
            typ: typ,
            expression: expression,
            value: value.into(),
        }
    }

    fn like<T>(self, pattern: T) -> Like
    where
        T: Into<String>;

    fn not_like<T>(self, pattern: T) -> Like
    where
        T: Into<String>;

    fn begins_with<T>(self, pattern: T) -> Like
    where
        T: Into<String>;

    fn not_begins_with<T>(self, pattern: T) -> Like
    where
        T: Into<String>;

    fn ends_into<T>(self, pattern: T) -> Like
    where
        T: Into<String>;

    fn not_ends_into<T>(self, pattern: T) -> Like
    where
        T: Into<String>;
}
