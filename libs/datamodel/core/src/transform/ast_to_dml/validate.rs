#![allow(clippy::suspicious_operation_groupings)] // clippy is wrong there

use crate::{
    ast,
    common::provider_names::MONGODB_SOURCE_NAME,
    configuration,
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
const RELATION_ATTRIBUTE_NAME: &str = "relation";

impl<'a> Validator<'a> {
    /// Creates a new instance, with all builtin attributes registered.
    pub fn new(source: Option<&'a configuration::Datasource>) -> Validator<'a> {
        Self { source }
    }

    pub(crate) fn validate(&self, ast: &ast::SchemaAst, schema: &dml::Datamodel, diagnostics: &mut Diagnostics) {
        for model in schema.models() {
            let ast_model = ast.find_model(&model.name).expect(STATE_ERROR);

            if let Err(ref mut the_errors) = self.validate_field_connector_specific(ast_model, model) {
                diagnostics.append(the_errors)
            }

            if let Err(ref mut the_errors) = self.validate_model_connector_specific(ast_model, model) {
                diagnostics.append(the_errors)
            }

            self.validate_referenced_fields_for_relation(schema, ast_model, model, diagnostics);
        }
    }

    fn validate_field_connector_specific(&self, ast_model: &ast::Model, model: &dml::Model) -> Result<(), Diagnostics> {
        let mut diagnostics = Diagnostics::new();

        if let Some(source) = self.source {
            let connector = &source.active_connector;
            for field in model.fields.iter() {
                let mut errors = Vec::new();
                connector.validate_field(field, &mut errors);

                for error in errors {
                    diagnostics.push_error(DatamodelError::ConnectorError {
                        message: error.to_string(),
                        span: ast_model.find_field_bang(field.name()).span,
                    });
                }
            }
        }

        diagnostics.to_result()
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

    fn validate_referenced_fields_for_relation(
        &self,
        datamodel: &dml::Datamodel,
        ast_model: &ast::Model,
        model: &dml::Model,
        errors: &mut Diagnostics,
    ) {
        // see https://github.com/prisma/prisma/issues/10105
        if self
            .source
            .as_ref()
            .map(|source| source.provider == MONGODB_SOURCE_NAME)
            .unwrap_or(false)
        {
            return;
        }

        for field in model.relation_fields() {
            let ast_field = match ast_model.find_field(&field.name) {
                Some(ast_field) => ast_field,
                None => continue, // skip relation fields created by reformatting
            };

            let rel_info = &field.relation_info;
            let related_model = datamodel.find_model(&rel_info.to).expect(STATE_ERROR);

            let fields_with_wrong_type = rel_info.fields.iter().zip(rel_info.references.iter())
                    .filter_map(|(base_field, referenced_field)| {
                        let base_field = model.find_field(base_field)?;
                        let referenced_field = related_model.find_field(referenced_field)?;

                        if base_field.field_type().is_compatible_with(&referenced_field.field_type()) {
                            return None
                        }

                        // Try harder to see if the final type is not the same.
                        // This check needs the connector, so it can't be in the dml
                        // crate.
                        if let Some(connector) = self.source.map(|source| &source.active_connector) {
                            let base_native_type = base_field.field_type().as_native_type().map(|(scalar, native)| (*scalar, native.serialized_native_type.clone())).or_else(|| -> Option<_> {
                                let field_type = base_field.field_type();
                                let scalar_type = field_type.as_scalar()?;

                                Some((*scalar_type, connector.default_native_type_for_scalar_type(scalar_type)))
                            });

                            let referenced_native_type = referenced_field.field_type().as_native_type().map(|(scalar, native)| (*scalar, native.serialized_native_type.clone())).or_else(|| -> Option<_> {
                                let field_type = referenced_field.field_type();
                                let scalar_type = field_type.as_scalar()?;

                                Some((*scalar_type, connector.default_native_type_for_scalar_type(scalar_type)))
                            });

                            if base_native_type.is_some() && base_native_type == referenced_native_type {
                                return None
                            }
                        }

                        Some(DatamodelError::new_attribute_validation_error(
                            &format!(
                                "The type of the field `{}` in the model `{}` is not matching the type of the referenced field `{}` in model `{}`.",
                                &base_field.name(),
                                &model.name,
                                &referenced_field.name(),
                                &related_model.name
                            ),
                            RELATION_ATTRIBUTE_NAME,
                            ast_field.span,
                        ))
                    });

            for err in fields_with_wrong_type {
                errors.push_error(err);
            }
        }
    }
}
