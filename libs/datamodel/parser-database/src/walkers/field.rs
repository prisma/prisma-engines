use super::{ModelWalker, RelationFieldWalker, ScalarFieldWalker};

#[derive(Copy, Clone)]
enum InnerWalker<'db> {
    Scalar(ScalarFieldWalker<'db>),
    Relation(RelationFieldWalker<'db>),
}

/// A model field, scalar or relation.
#[derive(Clone, Copy)]
pub struct FieldWalker<'db> {
    inner: InnerWalker<'db>,
}

impl<'db> FieldWalker<'db> {
    /// The field name.
    pub fn name(self) -> &'db str {
        match self.inner {
            InnerWalker::Scalar(f) => f.name(),
            InnerWalker::Relation(f) => f.name(),
        }
    }

    /// The model name.
    pub fn model(self) -> ModelWalker<'db> {
        match self.inner {
            InnerWalker::Scalar(f) => f.model(),
            InnerWalker::Relation(f) => f.model(),
        }
    }
}

impl<'db> From<ScalarFieldWalker<'db>> for FieldWalker<'db> {
    fn from(w: ScalarFieldWalker<'db>) -> Self {
        Self {
            inner: InnerWalker::Scalar(w),
        }
    }
}

impl<'db> From<RelationFieldWalker<'db>> for FieldWalker<'db> {
    fn from(w: RelationFieldWalker<'db>) -> Self {
        Self {
            inner: InnerWalker::Relation(w),
        }
    }
}
