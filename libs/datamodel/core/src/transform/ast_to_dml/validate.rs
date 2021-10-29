#![allow(clippy::suspicious_operation_groupings)] // clippy is wrong there

use crate::{
    ast, configuration,
    diagnostics::{DatamodelError, Diagnostics},
    dml,
};
use datamodel_connector::ConstraintType;

/// Helper for validating a datamodel.
///
/// When validating, we check if the datamodel is valid, and generate errors otherwise.
pub struct Validator<'a> {
    source: Option<&'a configuration::Datasource>,
}

/// State error message. Seeing this error means something went really wrong internally. It's the datamodel equivalent of a bluescreen.
const STATE_ERROR: &str = "Failed lookup of model, field or optional property during internal processing. This means that the internal representation was mutated incorrectly.";
const RELATION_ATTRIBUTE_NAME: &str = "relation";
const INDEX_ATTRIBUTE_NAME: &str = "index";
const UNIQUE_ATTRIBUTE_NAME: &str = "unique";

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

    pub(crate) fn post_standardisation_validate(
        &self,
        ast_schema: &ast::SchemaAst,
        datamodel: &dml::Datamodel,
        diagnostics: &mut Diagnostics,
    ) {
        let mut diagnostics_2 = self.validate_constraint_names_connector_specific(ast_schema, datamodel);
        diagnostics.append(&mut diagnostics_2);
    }

    fn validate_constraint_names_connector_specific(
        &self,
        ast_schema: &ast::SchemaAst,
        datamodel: &dml::Datamodel,
    ) -> Diagnostics {
        let mut diagnostics = Diagnostics::new();

        let source = if let Some(source) = self.source {
            source
        } else {
            return diagnostics;
        };

        let namespace_violations = source.active_connector.get_constraint_namespace_violations(datamodel);

        for model in datamodel.models() {
            let namespace_violation_scope = |name: &str, tpe: ConstraintType| {
                namespace_violations
                    .iter()
                    .find(|ns| ns.name == name && ns.tpe == tpe && model.name == ns.table)
                    .map(|ns| ns.scope)
            };
            let ast_model = ast_schema.find_model(&model.name).expect(STATE_ERROR);

            if let Some(pk) = &model.primary_key {
                if let Some(pk_name) = &pk.db_name {
                    if let Some(scope) = namespace_violation_scope(pk_name, ConstraintType::PrimaryKey) {
                        let span = ast_model.id_attribute().span;

                        let message = format!(
                            "The given constraint name `{}` has to be unique in the following namespace: {}. Please provide a different name using the `map` argument.",
                            pk_name,scope
                        );

                        let error = DatamodelError::new_attribute_validation_error(&message, "id", span);

                        diagnostics.push_error(error);
                    }
                }
            }

            for field in model.scalar_fields() {
                if let Some(df_name) = field.default_value().and_then(|d| d.db_name()) {
                    let ast_field = ast_model.find_field_bang(&field.name);

                    if let Some(scope) = namespace_violation_scope(df_name, ConstraintType::Default) {
                        let message = format!(
                            "The given constraint name `{}` has to be unique in the following namespace: {}. Please provide a different name using the `map` argument.",
                            df_name,scope
                        );

                        let span = ast_field.span_for_argument("default", "map").unwrap_or(ast_field.span);
                        let error = DatamodelError::new_attribute_validation_error(&message, "default", span);

                        diagnostics.push_error(error);
                    }
                }
            }

            for field in model.relation_fields() {
                let ast_field = ast_model
                    .fields
                    .iter()
                    .find(|ast_field| ast_field.name.name == field.name);

                let field_span = ast_field.map(|f| f.span).unwrap_or_else(ast::Span::empty);

                if let Some(fk_name) = field.relation_info.fk_name.as_ref() {
                    if let Some(scope) = namespace_violation_scope(fk_name, ConstraintType::ForeignKey) {
                        let span = ast_field
                            .and_then(|f| f.span_for_argument("relation", "map"))
                            .unwrap_or(field_span);

                        let message = format!(
                            "The given constraint name `{}` has to be unique in the following namespace: {}. Please provide a different name using the `map` argument.",
                            fk_name, scope
                        );

                        let error =
                            DatamodelError::new_attribute_validation_error(&message, RELATION_ATTRIBUTE_NAME, span);
                        diagnostics.push_error(error);
                    }
                }
            }

            for index in &model.indices {
                if let Some(idx_name) = &index.db_name {
                    if let Some(scope) = namespace_violation_scope(idx_name, ConstraintType::KeyOrIdx) {
                        let span = ast_model.span;
                        let message = format!(
                            "The given constraint name `{}` has to be unique in the following namespace: {}. Please provide a different name using the `map` argument.",
                            idx_name, scope
                        );

                        let error = DatamodelError::new_attribute_validation_error(
                            &message,
                            if index.is_unique() {
                                UNIQUE_ATTRIBUTE_NAME
                            } else {
                                INDEX_ATTRIBUTE_NAME
                            },
                            span,
                        );
                        diagnostics.push_error(error);
                    }
                }
            }
        }

        diagnostics
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
