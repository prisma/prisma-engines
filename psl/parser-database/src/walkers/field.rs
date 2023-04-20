use super::{CompositeTypeFieldWalker, ModelWalker, RelationFieldWalker, ScalarFieldWalker, Walker};
use crate::{
    types::{RelationField, ScalarField},
    ScalarType,
};
use schema_ast::ast;

/// A model field, scalar or relation.
pub type FieldWalker<'db> = Walker<'db, (ast::ModelId, ast::FieldId)>;

impl<'db> FieldWalker<'db> {
    /// The AST node for the field.
    pub fn ast_field(self) -> &'db ast::Field {
        &self.db.ast[self.id.0][self.id.1]
    }

    /// The field name.
    pub fn name(self) -> &'db str {
        self.ast_field().name()
    }

    /// Traverse the field's parent model.
    pub fn model(self) -> ModelWalker<'db> {
        self.walk(self.id.0)
    }

    /// Find out which kind of field this is.
    pub fn refine(self) -> RefinedFieldWalker<'db> {
        match self.db.types.refine_field(self.id) {
            either::Either::Left(id) => RefinedFieldWalker::Relation(self.walk(id)),
            either::Either::Right(id) => RefinedFieldWalker::Scalar(self.walk(id)),
        }
    }
}

/// A field that has been identified as scalar field or relation field.
#[derive(Copy, Clone)]
pub enum RefinedFieldWalker<'db> {
    /// A scalar field
    Scalar(ScalarFieldWalker<'db>),
    /// A relation field
    Relation(RelationFieldWalker<'db>),
}

impl<'db> From<ScalarFieldWalker<'db>> for FieldWalker<'db> {
    fn from(w: ScalarFieldWalker<'db>) -> Self {
        let ScalarField { model_id, field_id, .. } = w.db.types[w.id];
        Walker {
            db: w.db,
            id: (model_id, field_id),
        }
    }
}

impl<'db> From<RelationFieldWalker<'db>> for FieldWalker<'db> {
    fn from(w: RelationFieldWalker<'db>) -> Self {
        let RelationField { model_id, field_id, .. } = w.db.types[w.id];
        Walker {
            db: w.db,
            id: (model_id, field_id),
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
