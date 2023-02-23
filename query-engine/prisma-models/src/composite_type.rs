use crate::Field;
use psl::schema_ast::ast;

pub type CompositeType = crate::Zipper<ast::CompositeTypeId>;
pub type CompositeTypeRef = CompositeType;
pub type CompositeTypeWeakRef = CompositeType;

impl CompositeType {
    pub fn name(&self) -> &str {
        self.walker().name()
    }

    pub fn fields(&self) -> impl Iterator<Item = Field> + '_ {
        self.walker().fields().map(|f| Field::from((self.dm.clone(), f)))
    }

    pub fn find_field(&self, prisma_name: &str) -> Option<Field> {
        self.fields().into_iter().find(|f| f.name() == prisma_name)
    }

    pub fn find_field_by_db_name(&self, db_name: &str) -> Option<Field> {
        self.fields().into_iter().find(|f| f.db_name() == db_name)
    }
}
