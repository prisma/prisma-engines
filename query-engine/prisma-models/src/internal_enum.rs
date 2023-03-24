use crate::Zipper;

use psl::schema_ast::ast;

pub type InternalEnum = Zipper<ast::EnumId>;
pub type InternalEnumValue = Zipper<ast::EnumValueId>;

impl InternalEnum {
    pub fn name(&self) -> &str {
        self.dm.walk(self.id).name()
    }
}
