use crate::{Field, ast};
use psl::parser_database::CompositeTypeId;

pub type CompositeType = crate::Zipper<CompositeTypeId>;

impl CompositeType {
    pub fn name(&self) -> &str {
        self.walker().name()
    }

    pub fn fields(&self) -> impl Iterator<Item = Field> + '_ {
        self.walker()
            .fields()
            .filter(|f| !matches!(f.ast_field().field_type, ast::FieldType::Unsupported(..)))
            .map(|f| Field::from((self.dm.clone(), f)))
    }

    pub fn find_field(&self, prisma_name: &str) -> Option<Field> {
        self.fields().find(|f| f.name() == prisma_name)
    }

    pub fn find_field_by_db_name(&self, db_name: &str) -> Option<Field> {
        self.fields().find(|f| f.db_name() == db_name)
    }
}

impl std::fmt::Debug for CompositeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("CompositeType").field(&self.name()).finish()
    }
}
