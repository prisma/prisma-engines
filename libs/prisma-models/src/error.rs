use failure::Fail;
use prisma_value::ConversionFailure;

#[derive(Debug, Fail)]
pub enum DomainError {
    #[fail(display = "Model `{}` not found", name)]
    ModelNotFound { name: String },

    #[fail(display = "Field `{}` on model `{}` not found", name, model)]
    FieldNotFound { name: String, model: String },

    #[fail(display = "Relation `{}` not found", name)]
    RelationNotFound { name: String },

    #[fail(display = "ScalarField `{}` on model `{}` not found", name, model)]
    ScalarFieldNotFound { name: String, model: String },

    #[fail(display = "RelationField `{}` on model `{}` not found", name, model)]
    RelationFieldNotFound { name: String, model: String },

    #[fail(display = "Relation field `{}` on model `{}` not found", relation, model)]
    FieldForRelationNotFound { relation: String, model: String },

    #[fail(display = "Model id `{}` for relation `{}` not found", model_id, relation)]
    ModelForRelationNotFound { model_id: String, relation: String },

    #[fail(display = "Conversion from `{}` to `{}` failed.", _0, _1)]
    ConversionFailure(String, String),
}

impl From<super::ConversionFailure> for DomainError {
    fn from(err: ConversionFailure) -> Self {
        Self::ConversionFailure(err.from.to_owned(), err.to.to_owned())
    }
}
