use crate::ast::Expression;

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
    pub typ: LikeType,
    pub expression: Expression,
    pub value: String,
}

pub trait Likable {
    fn create_like<T>(typ: LikeType, expression: Expression, value: T) -> Like
    where
        T: Into<String>,
    {
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
