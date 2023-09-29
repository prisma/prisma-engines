use crate::Zipper;

use psl::schema_ast::ast;

pub type InternalEnum = Zipper<ast::EnumId>;
pub type InternalEnumValue = Zipper<ast::EnumValueId>;

impl InternalEnum {
    pub fn name(&self) -> &str {
        self.dm.walk(self.id).name()
    }

    pub fn db_name(&self) -> &str {
        self.dm.walk(self.id).database_name()
    }
}

impl std::fmt::Debug for InternalEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("InternalEnum").field(&self.name()).finish()
    }
}
