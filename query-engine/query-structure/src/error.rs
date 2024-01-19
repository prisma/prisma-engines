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

    #[error("Enum `{}` not found", name)]
    EnumNotFound { name: String },

    #[error("Conversion from `{}` to `{}` failed.", _0, _1)]
    ConversionFailure(String, String),
}
