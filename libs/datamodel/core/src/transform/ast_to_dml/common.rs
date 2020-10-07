use crate::messages::{DatamodelWarning, ErrorCollection};
use crate::{ast, dml, messages::DatamodelError};

/// State error message. Seeing this error means something went really wrong internally. It's the datamodel equivalent of a bluescreen.
pub (crate) const STATE_ERROR: &str = "Failed lookup of model or field during internal processing. This means that the internal representation was mutated incorrectly.";
pub (crate) const ERROR_GEN_STATE_ERROR: &str = "Failed lookup of model or field during generating an error message. This often means that a generated field or model was the cause of an error.";

impl ast::WithAttributes for Vec<ast::Attribute> {
    fn attributes(&self) -> &Vec<ast::Attribute> {
        self
    }
}

pub fn field_validation_error(
    message: &str,
    model: &dml::Model,
    field: &dml::Field,
    ast: &ast::SchemaAst,
) -> DatamodelError {
    DatamodelError::new_model_validation_error(
        message,
        &model.name,
        ast.find_field(&model.name, &field.name())
            .expect(ERROR_GEN_STATE_ERROR)
            .span,
    )
}

pub fn validate_preview_feature(
    preview_feature: &str,
    span: ast::Span,
    supported_preview_features: Vec<&str>,
    deprecated_preview_features: Vec<&str>,
) -> Result<(), DatamodelError> {
    if supported_preview_features.contains(&preview_feature) || deprecated_preview_features.contains(&preview_feature) {
        Ok(())
    } else {
        Err(DatamodelError::new_preview_feature_not_known_error(
            preview_feature,
            supported_preview_features,
            span,
        ))
    }
}
