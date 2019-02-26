use crate::ast::{ConditionTree, Table};

#[derive(Debug, PartialEq, Clone)]
pub struct JoinData {
    pub table: Table,
    pub conditions: ConditionTree,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Join {
    Inner(JoinData),
}

pub trait Joinable {
    fn on<T>(self, conditions: T) -> JoinData
    where
        T: Into<ConditionTree>;
}

macro_rules! joinable {
    ($($kind:ty),*) => (
        $(
            impl Joinable for $kind {
                fn on<T>(self, conditions: T) -> JoinData
                where
                    T: Into<ConditionTree>,
                {
                    JoinData {
                        table: self.into(),
                        conditions: conditions.into(),
                    }
                }
            }
        )*
    );
}

joinable!(String, (String, String));
joinable!(&str, (&str, &str));
