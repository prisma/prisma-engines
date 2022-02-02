use indexmap::IndexSet;
use schema_ast::ast;
use std::ops::Index;

#[derive(Default)]
pub(crate) struct StringInterner {
    store: IndexSet<String>,
}

#[derive(PartialEq, Debug, Clone, Copy, Hash, Eq, Ord, PartialOrd)]
pub(crate) struct InternedString(usize);

impl StringInterner {
    pub(crate) fn intern(&mut self, s: &str) -> InternedString {
        if let Some(idx) = self.store.get_index_of(s) {
            InternedString(idx)
        } else {
            let (idx, _) = self.store.insert_full(s.to_owned());
            InternedString(idx)
        }
    }
}

impl Index<InternedString> for StringInterner {
    type Output = str;

    fn index(&self, index: InternedString) -> &Self::Output {
        &self.store[index.0]
    }
}

/// A maybe-allocated AST string.
#[derive(Debug, Clone)]
pub(crate) struct AstString {
    #[allow(unused)] // we will use it.
    pub(crate) span: ast::Span,
    pub(crate) value: InternedString,
}
