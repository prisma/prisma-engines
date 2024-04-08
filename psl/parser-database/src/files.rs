use crate::FileId;
use schema_ast::ast;
use std::ops::Index;

/// The content is a list of (file path, file source text, file AST).
///
/// The file path can be anything, the PSL implementation will only use it to display the file name
/// in errors. For example, files can come from nested directories.
pub(crate) struct Files(pub(super) Vec<(String, schema_ast::SourceFile, ast::SchemaAst)>);

impl Files {
    pub(crate) fn iter(&self) -> impl Iterator<Item = (FileId, &String, &schema_ast::SourceFile, &ast::SchemaAst)> {
        self.0
            .iter()
            .enumerate()
            .map(|(idx, (path, contents, ast))| (FileId(idx as u32), path, contents, ast))
    }
}

impl Index<crate::FileId> for Files {
    type Output = (String, schema_ast::SourceFile, ast::SchemaAst);

    fn index(&self, index: crate::FileId) -> &Self::Output {
        &self.0[index.0 as usize]
    }
}

impl<Id> Index<crate::InFile<Id>> for Files
where
    ast::SchemaAst: Index<Id>,
{
    type Output = <ast::SchemaAst as Index<Id>>::Output;

    fn index(&self, index: crate::InFile<Id>) -> &Self::Output {
        &self[index.0].2[index.1]
    }
}
