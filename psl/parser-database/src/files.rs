use crate::FileId;
use schema_ast::ast;
use std::ops::Index;

pub(crate) struct Files(pub(super) Vec<(FileId, ast::SchemaAst)>);

impl Index<crate::FileId> for Files {
    type Output = ast::SchemaAst;

    fn index(&self, index: crate::FileId) -> &Self::Output {
        &self.0[index.0].1
    }
}

impl<Id> Index<crate::InFile<Id>> for Files
where
    ast::SchemaAst: Index<Id>,
{
    type Output = <ast::SchemaAst as Index<Id>>::Output;

    fn index(&self, index: crate::InFile<Id>) -> &Self::Output {
        &self[index.0][index.1]
    }
}
