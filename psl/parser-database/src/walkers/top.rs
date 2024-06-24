use crate::{
    ast::{self, WithSpan},
    FileId,
};

/// Any top declaration in the Prisma schema.
pub type TopWalker<'db> = super::Walker<'db, crate::TopId>;

impl<'db> TopWalker<'db> {
    /// The name of the model.
    pub fn name(self) -> &'db str {
        self.ast_top().name()
    }

    /// The ID of the file containing the model.
    pub fn file_id(self) -> FileId {
        self.id.0
    }

    /// Is the model defined in a specific file?
    pub fn is_defined_in_file(self, file_id: FileId) -> bool {
        self.ast_top().span().file_id == file_id
    }

    /// The AST node.
    pub fn ast_top(self) -> &'db ast::Top {
        &self.db.asts[self.id]
    }
}
