use schema_ast::ast;

/// The stable identifier for a PSL file.
#[derive(Debug, PartialEq, Clone, Copy, Hash, Eq, PartialOrd, Ord)]
pub struct FileId(pub(crate) usize);

impl FileId {
    pub(crate) const ZERO: FileId = FileId(0);
    pub(crate) const MAX: FileId = FileId(usize::MAX);
}

/// An AST identifier with the accompanyin file ID.
pub type InFile<Id> = (FileId, Id);

/// See [ast::ModelId]
pub type ModelId = InFile<ast::ModelId>;

/// See [ast::EnumId]
pub type EnumId = InFile<ast::EnumId>;

/// See [ast::CompositeTypeId]
pub type CompositeTypeId = InFile<ast::CompositeTypeId>;

/// See [ast::TopId]
pub type TopId = InFile<ast::TopId>;

/// See [ast::AttributeId]
pub type AttributeId = InFile<ast::AttributeId>;

/// See [ast::AttributeContainer]
pub type AttributeContainer = InFile<ast::AttributeContainer>;
