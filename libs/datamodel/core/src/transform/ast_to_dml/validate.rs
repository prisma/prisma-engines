use crate::{
    ast, configuration,
    diagnostics::{DatamodelError, Diagnostics},
    dml,
};

/// Helper for validating a datamodel.
///
/// When validating, we check if the datamodel is valid, and generate errors otherwise.
pub struct Validator<'a> {
    source: Option<&'a configuration::Datasource>,
}

/// State error message. Seeing this error means something went really wrong internally. It's the datamodel equivalent of a bluescreen.
const STATE_ERROR: &str = "Failed lookup of model, field or optional property during internal processing. This means that the internal representation was mutated incorrectly.";

impl<'a> Validator<'a> {
    /// Creates a new instance, with all builtin attributes registered.
    pub fn new(source: Option<&'a configuration::Datasource>) -> Validator<'a> {
        Self { source }
    }

    pub(crate) fn validate(&self, ast: &ast::SchemaAst, schema: &dml::Datamodel, diagnostics: &mut Diagnostics) {
        for model in schema.models() {
            let ast_model = ast.find_model(&model.name).expect(STATE_ERROR);

            if let Err(ref mut the_errors) = self.validate_model_connector_specific(ast_model, model) {
                diagnostics.append(the_errors)
            }
        }
    }

    fn validate_model_connector_specific(&self, ast_model: &ast::Model, model: &dml::Model) -> Result<(), Diagnostics> {
        let mut diagnostics = Diagnostics::new();

        if let Some(source) = self.source {
            let connector = &source.active_connector;
            let mut errors = Vec::new();

            connector.validate_model(model, &mut errors);

            for error in errors {
                diagnostics.push_error(DatamodelError::new_connector_error(&error.to_string(), ast_model.span))
            }
        }

        diagnostics.to_result()
    }
}
