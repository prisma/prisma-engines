use crate::{
    psl::{parser_database::walkers::Walker, schema_ast::ast},
    InternalDataModelRef,
};
use std::{
    fmt,
    hash::{Hash, Hasher},
};

// Invariant: InternalDataModel must not contain any Zipper, this would be a reference counting
// cycle (memory leak).
#[derive(Clone)]
pub struct Zipper<I> {
    pub id: I,
    pub dm: InternalDataModelRef,
}

impl<I: fmt::Debug> fmt::Debug for Zipper<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.id.fmt(f)
    }
}

impl<I: PartialEq> PartialEq for Zipper<I> {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

impl<I: Eq> Eq for Zipper<I> {}

impl<I: Copy> Zipper<I> {
    pub fn walker(&self) -> Walker<'_, I> {
        self.dm.schema.db.walk(self.id)
    }
}

impl<I: Hash> Hash for Zipper<I> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

pub type InternalEnum = Zipper<ast::EnumId>;
pub type InternalEnumRef = InternalEnum;
pub type InternalEnumValue = Zipper<ast::EnumValueId>;
