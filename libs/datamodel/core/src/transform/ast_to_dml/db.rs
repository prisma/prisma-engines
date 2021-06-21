mod names;

use crate::{ast, diagnostics::Diagnostics};

pub(crate) struct ParserDatabase<'a> {
    ast: &'a ast::SchemaAst,
    names: names::Names<'a>,
}

impl<'ast> ParserDatabase<'ast> {
    pub(super) fn new(ast: &'ast ast::SchemaAst, diagnostics: &mut Diagnostics) -> Self {
        let names = names::Names::new(ast, diagnostics);

        ParserDatabase { ast, names }
    }

    pub(super) fn ast(&self) -> &'ast ast::SchemaAst {
        self.ast
    }

    pub(crate) fn iter_enums(&self) -> impl Iterator<Item = (ast::TopId, &'ast ast::Enum)> + '_ {
        self.names
            .tops
            .values()
            .filter_map(move |topid| self.ast[*topid].as_enum().map(|enm| (*topid, enm)))
    }

    pub(super) fn get_enum(&self, name: &str) -> Option<&'ast ast::Enum> {
        self.names.tops.get(name).and_then(|top_id| self.ast[*top_id].as_enum())
    }
}
