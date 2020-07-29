use prisma_value::ConversionFailure;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Model `{}` not found", name)]
    ModelNotFound { name: String },

    #[error("Field `{}` on model `{}` not found", name, model)]
    FieldNotFound { name: String, model: String },

    #[error("Relation `{}` not found", name)]
    RelationNotFound { name: String },

    #[error("ScalarField `{}` on model `{}` not found", name, model)]
    ScalarFieldNotFound { name: String, model: String },

    #[error("RelationField `{}` on model `{}` not found", name, model)]
    RelationFieldNotFound { name: String, model: String },

    #[error("Relation field `{}` on model `{}` not found", relation, model)]
    FieldForRelationNotFound { relation: String, model: String },

    #[error("Model id `{}` for relation `{}` not found", model_id, relation)]
    ModelForRelationNotFound { model_id: String, relation: String },

    #[error("Conversion from `{}` to `{}` failed.", _0, _1)]
    ConversionFailure(String, String),
}

impl From<super::ConversionFailure> for DomainError {
    fn from(err: ConversionFailure) -> Self {
        Self::ConversionFailure(err.from.to_owned(), err.to.to_owned())
    }
}
