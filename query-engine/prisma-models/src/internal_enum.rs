use crate::Zipper;
use psl::{parser_database::EnumId, schema_ast::ast::EnumValueId};

pub type InternalEnum = Zipper<EnumId>;
pub type InternalEnumValue = Zipper<EnumValueId>;

impl InternalEnum {
    pub fn name(&self) -> &str {
        self.dm.walk(self.id).name()
    }
}

impl std::fmt::Debug for InternalEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("InternalEnum").field(&self.name()).finish()
    }
}
