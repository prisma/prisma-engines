use crate::Zipper;
use psl::{parser_database::EnumId, schema_ast::ast::EnumValueId};

pub type InternalEnum = Zipper<EnumId>;
pub type InternalEnumValue = Zipper<EnumValueId>;

impl InternalEnum {
    pub fn name(&self) -> &str {
        self.dm.walk(self.id).name()
    }

    pub fn db_name(&self) -> &str {
        self.dm.walk(self.id).database_name()
    }

    pub fn schema_name(&self) -> Option<&str> {
        self.dm.walk(self.id).schema().map(|tuple| tuple.0)
    }
}

impl std::fmt::Debug for InternalEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("InternalEnum").field(&self.name()).finish()
    }
}
