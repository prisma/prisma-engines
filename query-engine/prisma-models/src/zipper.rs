use crate::{
    psl::{parser_database::walkers::Walker, schema_ast::ast},
    InternalDataModelRef,
};

// Invariant: InternalDataModel must not contain any Zipper, this would be a reference counting
// cycle (memory leak).
#[derive(Debug, Clone)]
pub struct Zipper<I> {
    pub id: I,
    pub dm: InternalDataModelRef,
}

impl<I: PartialEq> PartialEq for Zipper<I> {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

impl<I: Copy> Zipper<I> {
    pub fn walker(&self) -> Walker<'_, I> {
        self.dm.schema.db.walk(self.id)
    }
}

pub type InternalEnum = Zipper<ast::EnumId>;
pub type InternalEnumRef = InternalEnum;
pub type InternalEnumValue = Zipper<ast::EnumValueId>;
