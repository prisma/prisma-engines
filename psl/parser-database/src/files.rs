use crate::FileId;
use diagnostics::Diagnostics;
use schema_ast::ast;
use std::ops::Index;

/// The content is a list of (file path, file source text, file AST).
///
/// The file path can be anything, the PSL implementation will only use it to display the file name
/// in errors. For example, files can come from nested directories.
pub struct Files(pub Vec<(String, schema_ast::SourceFile, ast::SchemaAst)>);

impl Files {
    /// Create a new Files instance from multiple files.
    pub fn new(files: &[(String, schema_ast::SourceFile)], diagnostics: &mut Diagnostics) -> Self {
        let asts = files
            .iter()
            .enumerate()
            .map(|(file_idx, (path, source))| {
                let id = FileId(file_idx as u32);
                let ast = schema_ast::parse_schema(source.as_str(), diagnostics, id);
                (path.to_owned(), source.clone(), ast)
            })
            .collect();
        Self(asts)
    }

    /// Iterate all parsed files.
    #[allow(clippy::should_implement_trait)]
    pub fn iter(&self) -> impl Iterator<Item = (FileId, &String, &schema_ast::SourceFile, &ast::SchemaAst)> {
        self.0
            .iter()
            .enumerate()
            .map(|(idx, (path, contents, ast))| (FileId(idx as u32), path, contents, ast))
    }

    /// Iterate all parsed files, consuming the parser database.
    #[allow(clippy::should_implement_trait)]
    pub fn into_iter(self) -> impl Iterator<Item = (FileId, String, schema_ast::SourceFile, ast::SchemaAst)> {
        self.0
            .into_iter()
            .enumerate()
            .map(|(idx, (path, contents, ast))| (FileId(idx as u32), path, contents, ast))
    }

    /// Render the given diagnostics (warnings + errors) into a String.
    /// This method is multi-file aware.
    pub fn render_diagnostics(&self, diagnostics: &Diagnostics) -> String {
        let mut out = Vec::new();

        for error in diagnostics.errors() {
            let (file_name, source, _) = &self[error.span().file_id];
            error.pretty_print(&mut out, file_name, source.as_str()).unwrap();
        }

        String::from_utf8(out).unwrap()
    }

    /// Returns the number of files.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.0.len()
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
