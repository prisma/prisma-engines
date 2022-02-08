use super::{ModelWalker, RelationFieldWalker, ScalarFieldWalker};

#[derive(Copy, Clone)]
enum InnerWalker<'ast, 'db> {
    Scalar(ScalarFieldWalker<'ast, 'db>),
    Relation(RelationFieldWalker<'ast, 'db>),
}

/// A model field, scalar or relation.
#[derive(Clone, Copy)]
pub struct FieldWalker<'ast, 'db> {
    inner: InnerWalker<'ast, 'db>,
}

impl<'ast, 'db> FieldWalker<'ast, 'db> {
    /// The field name.
    pub fn name(self) -> &'db str {
        match self.inner {
            InnerWalker::Scalar(f) => f.name(),
            InnerWalker::Relation(f) => f.name(),
        }
    }

    /// The model name.
    pub fn model(self) -> ModelWalker<'ast, 'db> {
        match self.inner {
            InnerWalker::Scalar(f) => f.model(),
            InnerWalker::Relation(f) => f.model(),
        }
    }
}

impl<'ast, 'db> From<ScalarFieldWalker<'ast, 'db>> for FieldWalker<'ast, 'db> {
    fn from(w: ScalarFieldWalker<'ast, 'db>) -> Self {
        Self {
            inner: InnerWalker::Scalar(w),
        }
    }
}

impl<'ast, 'db> From<RelationFieldWalker<'ast, 'db>> for FieldWalker<'ast, 'db> {
    fn from(w: RelationFieldWalker<'ast, 'db>) -> Self {
        Self {
            inner: InnerWalker::Relation(w),
        }
    }
}
