use crate::FileId;
use schema_ast::ast;
use std::{collections::HashMap, ops::Index};

pub(crate) struct Files(pub(super) HashMap<FileId, ast::SchemaAst>);

impl Index<crate::FileId> for Files {
    type Output = ast::SchemaAst;

    fn index(&self, index: crate::FileId) -> &Self::Output {
        &self.0[&index]
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
