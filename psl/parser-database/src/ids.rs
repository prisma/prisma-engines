use schema_ast::ast;

#[derive(Debug, PartialEq, Clone, Copy, Hash, Eq, PartialOrd, Ord)]
pub struct SchemaId(pub(crate) usize);

impl SchemaId {
    pub(crate) const ZERO: SchemaId = SchemaId(0);
    pub(crate) const MAX: SchemaId = SchemaId(usize::MAX);
}

pub type InFile<Id> = (SchemaId, Id);

pub(crate) type ModelId = InFile<ast::ModelId>;
pub(crate) type EnumId = InFile<ast::EnumId>;
pub(crate) type CompositeTypeId = InFile<ast::CompositeTypeId>;
pub(crate) type TopId = InFile<ast::TopId>;
pub(crate) type AttributeId = InFile<ast::AttributeId>;
pub(crate) type AttributeContainer = InFile<ast::AttributeContainer>;
