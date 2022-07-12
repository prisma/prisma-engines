use super::{CompositeTypeFieldWalker, ModelWalker, RelationFieldWalker, ScalarFieldWalker};
use crate::ScalarType;
use schema_ast::ast;

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

#[derive(Copy, Clone)]
enum InnerTypedFieldWalker<'db> {
    Scalar(ScalarFieldWalker<'db>),
    Composite(CompositeTypeFieldWalker<'db>),
}

impl<'db> From<ScalarFieldWalker<'db>> for TypedFieldWalker<'db> {
    fn from(w: ScalarFieldWalker<'db>) -> Self {
        Self {
            inner: InnerTypedFieldWalker::Scalar(w),
        }
    }
}

impl<'db> From<CompositeTypeFieldWalker<'db>> for TypedFieldWalker<'db> {
    fn from(w: CompositeTypeFieldWalker<'db>) -> Self {
        Self {
            inner: InnerTypedFieldWalker::Composite(w),
        }
    }
}

/// A model or composite type field of a scalar type.
#[derive(Clone, Copy)]
pub struct TypedFieldWalker<'db> {
    inner: InnerTypedFieldWalker<'db>,
}

impl<'db> TypedFieldWalker<'db> {
    /// The type of the field in case it is a scalar type (not an enum, not a composite type).
    pub fn scalar_type(self) -> Option<ScalarType> {
        match self.inner {
            InnerTypedFieldWalker::Scalar(field) => field.scalar_type(),
            InnerTypedFieldWalker::Composite(field) => field.scalar_type(),
        }
    }

    /// (attribute scope, native type name, arguments, span)
    ///
    /// For example: `@db.Text` would translate to ("db", "Text", &[], <the span>)
    pub fn raw_native_type(self) -> Option<(&'db str, &'db str, &'db [String], ast::Span)> {
        match self.inner {
            InnerTypedFieldWalker::Scalar(sf) => sf.raw_native_type(),
            InnerTypedFieldWalker::Composite(cf) => cf.raw_native_type(),
        }
    }
}
