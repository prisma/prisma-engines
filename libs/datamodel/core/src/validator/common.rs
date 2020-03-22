use crate::{ast, dml, error::DatamodelError};

/// State error message. Seeing this error means something went really wrong internally. It's the datamodel equivalent of a bluescreen.
pub (crate) const STATE_ERROR: &str = "Failed lookup of model or field during internal processing. This means that the internal representation was mutated incorrectly.";
pub (crate) const ERROR_GEN_STATE_ERROR: &str = "Failed lookup of model or field during generating an error message. This often means that a generated field or model was the cause of an error.";

impl ast::WithDirectives for Vec<ast::Directive> {
    fn directives(&self) -> &Vec<ast::Directive> {
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
        ast.find_field(&model.name, &field.name)
            .expect(ERROR_GEN_STATE_ERROR)
            .span,
    )
}

pub fn tie(
    a_model: &dml::Model,
    a_field: &dml::Field,
    b_model: &dml::Model,
    b_field: &dml::Field,
) -> bool {
    // Model with lower name wins, if name is equal fall back to field.
    a_model.name < b_model.name || (a_model.name == b_model.name && a_field.name < b_field.name)
}

#[allow(unused)]
pub fn tie_str(a_model: &str, a_field: &str, b_model: &str, b_field: &str) -> bool {
    // Model with lower name wins, if name is equal fall back to field.
    a_model < b_model || (a_model == b_model && a_field < b_field)
}
