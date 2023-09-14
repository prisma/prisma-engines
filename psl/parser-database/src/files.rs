use crate::SchemaId;
use schema_ast::ast;
use std::{collections::HashMap, ops::Index};

pub(crate) struct Files(pub(super) HashMap<SchemaId, ast::SchemaAst>);

impl Index<crate::SchemaId> for Files {
    type Output = ast::SchemaAst;

    fn index(&self, index: crate::SchemaId) -> &Self::Output {
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
