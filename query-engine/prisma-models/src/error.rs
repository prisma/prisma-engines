use prisma_value::ConversionFailure;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum DomainError {
    #[error("Model `{}` not found", name)]
    ModelNotFound { name: String },

    #[error("Field `{}` on {} `{}` not found", name, container_type, container_name)]
    FieldNotFound {
        name: String,
        container_type: &'static str,
        container_name: String,
    },

    #[error("Relation `{}` not found", name)]
    RelationNotFound { name: String },

    #[error("ScalarField `{}` on {} `{}` not found", name, container_type, container_name)]
    ScalarFieldNotFound {
        name: String,
        container_name: String,
        container_type: &'static str,
    },

    #[error("CompositeField `{}` on {} `{}` not found", name, container_type, container_name)]
    CompositeFieldNotFound {
        name: String,
        container_name: String,
        container_type: &'static str,
    },

    #[error("RelationField `{}` on model `{}` not found", name, model)]
    RelationFieldNotFound { name: String, model: String },

    #[error("Relation field `{}` on model `{}` not found", relation, model)]
    FieldForRelationNotFound { relation: String, model: String },

    #[error("Model id `{}` for relation `{}` not found", model_id, relation)]
    ModelForRelationNotFound { model_id: String, relation: String },

    #[error("Enum `{}` not found", name)]
    EnumNotFound { name: String },

    #[error("Conversion from `{}` to `{}` failed.", _0, _1)]
    ConversionFailure(String, String),
}

impl From<super::ConversionFailure> for DomainError {
    fn from(err: ConversionFailure) -> Self {
        Self::ConversionFailure(err.from.into_owned(), err.to.into_owned())
    }
}
