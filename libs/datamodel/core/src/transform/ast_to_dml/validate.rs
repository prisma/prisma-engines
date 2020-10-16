use crate::ast::WithAttributes;
use crate::{
    ast, configuration,
    diagnostics::{DatamodelError, Diagnostics},
    dml, DefaultValue, FieldType,
};
use prisma_value::PrismaValue;
use std::collections::HashSet;

/// Helper for validating a datamodel.
///
/// When validating, we check if the datamodel is valid, and generate errors otherwise.
pub struct Validator<'a> {
    source: Option<&'a configuration::Datasource>,
}

/// State error message. Seeing this error means something went really wrong internally. It's the datamodel equivalent of a bluescreen.
const STATE_ERROR: &str = "Failed lookup of model, field or optional property during internal processing. This means that the internal representation was mutated incorrectly.";
const RELATION_ATTRIBUTE_NAME: &str = "relation";
const RELATION_ATTRIBUTE_NAME_WITH_AT: &str = "@relation";
const PRISMA_FORMAT_HINT: &str = "You can run `prisma format` to fix this automatically.";

impl<'a> Validator<'a> {
    /// Creates a new instance, with all builtin attributes registered.
    pub fn new(source: Option<&'a configuration::Datasource>) -> Validator<'a> {
        Self { source }
    }

    pub fn validate(&self, ast_schema: &ast::SchemaAst, schema: &mut dml::Datamodel) -> Result<(), Diagnostics> {
        let mut all_errors = Diagnostics::new();

        if let Err(ref mut errs) = self.validate_names(ast_schema) {
            all_errors.append(errs);
        }

        if let Err(ref mut errs) = self.validate_names_for_indexes(ast_schema, schema) {
            all_errors.append(errs);
        }

        // Model level validations.
        for model in schema.models() {
            // Having a separate error collection allows checking whether any error has occurred for a model.
            let mut errors_for_model = Diagnostics::new();

            if let Err(err) = self.validate_model_has_strict_unique_criteria(
                ast_schema.find_model(&model.name).expect(STATE_ERROR),
                model,
            ) {
                errors_for_model.push_error(err);
            }
            if let Err(err) = self.validate_model_name(ast_schema.find_model(&model.name).expect(STATE_ERROR), model) {
                errors_for_model.push_error(err);
            }

            if let Err(err) = self.validate_relations_not_ambiguous(ast_schema, model) {
                errors_for_model.push_error(err);
            }

            if let Err(err) = self.validate_embedded_types_have_no_back_relation(ast_schema, schema, model) {
                errors_for_model.push_error(err);
            }

            if let Err(ref mut the_errors) =
                self.validate_field_arities(ast_schema.find_model(&model.name).expect(STATE_ERROR), model)
            {
                errors_for_model.append(the_errors);
            }

            if let Err(ref mut the_errors) =
                self.validate_field_types(ast_schema.find_model(&model.name).expect(STATE_ERROR), model)
            {
                errors_for_model.append(the_errors);
            }

            if let Err(ref mut the_errors) =
                self.validate_field_connector_specific(ast_schema.find_model(&model.name).expect(STATE_ERROR), model)
            {
                errors_for_model.append(the_errors)
            }

            if let Err(ref mut the_errors) =
                self.validate_enum_default_values(schema, ast_schema.find_model(&model.name).expect(STATE_ERROR), model)
            {
                errors_for_model.append(the_errors);
            }

            if let Err(ref mut the_errors) =
                self.validate_auto_increment(ast_schema.find_model(&model.name).expect(STATE_ERROR), model)
            {
                errors_for_model.append(the_errors);
            }

            if let Err(ref mut the_errors) = self.validate_base_fields_for_relation(
                schema,
                ast_schema.find_model(&model.name).expect(STATE_ERROR),
                model,
            ) {
                errors_for_model.append(the_errors);
            }

            if let Err(ref mut the_errors) = self.validate_referenced_fields_for_relation(
                schema,
                ast_schema.find_model(&model.name).expect(STATE_ERROR),
                model,
            ) {
                errors_for_model.append(the_errors);
            }

            //            if !errors_for_model.has_errors() {
            //                let mut new_errors = self.validate_relation_arguments_bla(
            //                    schema,
            //                    ast_schema.find_model(&model.name).expect(STATE_ERROR),
            //                    model,
            //                );
            //                errors_for_model.append(&mut new_errors);
            //            }

            all_errors.append(&mut errors_for_model);
        }

        // Enum level validations.
        for declared_enum in schema.enums() {
            let mut errors_for_enum = Diagnostics::new();
            if let Err(err) = self.validate_enum_name(
                ast_schema.find_enum(&declared_enum.name).expect(STATE_ERROR),
                declared_enum,
            ) {
                errors_for_enum.push_error(err);
            }

            all_errors.append(&mut errors_for_enum);
        }

        if all_errors.has_errors() {
            Err(all_errors)
        } else {
            Ok(())
        }
    }

    pub fn post_standardisation_validate(
        &self,
        ast_schema: &ast::SchemaAst,
        schema: &mut dml::Datamodel,
    ) -> Result<(), Diagnostics> {
        let mut all_errors = Diagnostics::new();

        // Model level validations.
        for model in schema.models() {
            // Having a separate error collection allows checking whether any error has occurred for a model.
            let mut errors_for_model = Diagnostics::new();

            if !errors_for_model.has_errors() {
                let mut new_errors = self.validate_relation_arguments_bla(
                    schema,
                    ast_schema.find_model(&model.name).expect(STATE_ERROR),
                    model,
                );
                errors_for_model.append(&mut new_errors);
            }

            all_errors.append(&mut errors_for_model);
        }

        if all_errors.has_errors() {
            Err(all_errors)
        } else {
            Ok(())
        }
    }

    fn validate_names(&self, ast_schema: &ast::SchemaAst) -> Result<(), Diagnostics> {
        let mut errors = Diagnostics::new();

        for model in ast_schema.models() {
            errors.push_opt_error(model.name.validate("Model").err());
            errors.append(&mut model.validate_attributes());

            for field in model.fields.iter() {
                errors.push_opt_error(field.name.validate("Field").err());
                errors.append(&mut field.validate_attributes());
            }
        }

        for enum_decl in ast_schema.enums() {
            errors.push_opt_error(enum_decl.name.validate("Enum").err());
            errors.append(&mut enum_decl.validate_attributes());

            for enum_value in enum_decl.values.iter() {
                errors.push_opt_error(enum_value.name.validate("Enum Value").err());
                errors.append(&mut enum_value.validate_attributes());
            }
        }

        errors.to_result()
    }

    fn validate_names_for_indexes(
        &self,
        ast_schema: &ast::SchemaAst,
        schema: &dml::Datamodel,
    ) -> Result<(), Diagnostics> {
        let mut errors = Diagnostics::new();
        let mut index_names = HashSet::new();

        let multiple_indexes_with_same_name_are_supported = self
            .source
            .map(|source| source.combined_connector.supports_multiple_indexes_with_same_name())
            .unwrap_or(false);

        for model in schema.models() {
            if let Some(ast_model) = ast_schema.find_model(&model.name) {
                for index in model.indices.iter() {
                    if let Some(index_name) = &index.name {
                        if index_names.contains(index_name) && !multiple_indexes_with_same_name_are_supported {
                            let ast_index = ast_model
                                .attributes
                                .iter()
                                .find(|attribute| attribute.name.name == "index")
                                .unwrap();
                            errors.push_error(DatamodelError::new_multiple_indexes_with_same_name_are_not_supported(
                                index_name,
                                ast_index.span,
                            ));
                        }
                        index_names.insert(index_name);
                    }
                }
            }
        }

        errors.to_result()
    }

    fn validate_field_arities(&self, ast_model: &ast::Model, model: &dml::Model) -> Result<(), Diagnostics> {
        let mut errors = Diagnostics::new();

        // TODO: this is really ugly
        let scalar_lists_are_supported = match self.source {
            Some(source) => source.combined_connector.supports_scalar_lists(),
            None => false,
        };

        for field in model.scalar_fields() {
            if field.is_list() && !scalar_lists_are_supported {
                errors.push_error(DatamodelError::new_scalar_list_fields_are_not_supported(
                    &model.name,
                    &field.name,
                    ast_model.find_field(&field.name).span,
                ));
            }
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(())
        }
    }

    fn validate_field_types(&self, ast_model: &ast::Model, model: &dml::Model) -> Result<(), Diagnostics> {
        let mut errors = Diagnostics::new();

        for field in model.scalar_fields() {
            if let Some(dml::ScalarType::Json) = field.field_type.scalar_type() {
                // TODO: this is really ugly
                let supports_json_type = match self.source {
                    Some(source) => source.combined_connector.supports_json(),
                    None => false,
                };
                if !supports_json_type {
                    errors.push_error(DatamodelError::new_field_validation_error(
                        &format!("Field `{}` in model `{}` can't be of type Json. The current connector does not support the Json type.", &field.name, &model.name),
                        &model.name,
                        &field.name,
                        ast_model.find_field(&field.name).span,
                    ));
                }
            }
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(())
        }
    }

    fn validate_enum_default_values(
        &self,
        data_model: &dml::Datamodel,
        ast_model: &ast::Model,
        model: &dml::Model,
    ) -> Result<(), Diagnostics> {
        let mut errors = Diagnostics::new();

        for field in model.scalar_fields() {
            if let Some(DefaultValue::Single(PrismaValue::Enum(enum_value))) = &field.default_value {
                if let FieldType::Enum(enum_name) = &field.field_type {
                    if let Some(dml_enum) = data_model.find_enum(&enum_name) {
                        if !dml_enum.values.iter().any(|value| &value.name == enum_value) {
                            errors.push_error(DatamodelError::new_attribute_validation_error(
                                &"The defined default value is not a valid value of the enum specified for the field."
                                    .to_string(),
                                "default",
                                ast_model.find_field(&field.name).span,
                            ))
                        }
                    }
                }
            }
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(())
        }
    }

    fn validate_auto_increment(&self, ast_model: &ast::Model, model: &dml::Model) -> Result<(), Diagnostics> {
        let mut errors = Diagnostics::new();

        if let Some(data_source) = self.source {
            if !data_source.combined_connector.supports_multiple_auto_increment()
                && model.auto_increment_fields().count() > 1
            {
                errors.push_error(DatamodelError::new_attribute_validation_error(
                    &"The `autoincrement()` default value is used multiple times on this model even though the underlying datasource only supports one instance per table.".to_string(),
                    "default",
                    ast_model.span,
                ))
            }

            // go over all fields
            for field in model.scalar_fields() {
                let ast_field = ast_model.find_field(&field.name);

                if !field.is_id
                    && field.is_auto_increment()
                    && !data_source.combined_connector.supports_non_id_auto_increment()
                {
                    errors.push_error(DatamodelError::new_attribute_validation_error(
                    &"The `autoincrement()` default value is used on a non-id field even though the datasource does not support this.".to_string(),
                    "default",
                    ast_field.span,
                ))
                }

                if field.is_auto_increment()
                    && !model.field_is_indexed(&field.name)
                    && !data_source.combined_connector.supports_non_indexed_auto_increment()
                {
                    errors.push_error(DatamodelError::new_attribute_validation_error(
                    &"The `autoincrement()` default value is used on a non-indexed field even though the datasource does not support this.".to_string(),
                    "default",
                    ast_field.span,
                ))
                }
            }
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(())
        }
    }

    fn validate_model_has_strict_unique_criteria(
        &self,
        ast_model: &ast::Model,
        model: &dml::Model,
    ) -> Result<(), DatamodelError> {
        let multiple_single_field_id_error = Err(DatamodelError::new_model_validation_error(
            "At most one field must be marked as the id field with the `@id` attribute.",
            &model.name,
            ast_model.span,
        ));

        let multiple_id_criteria_error = Err(DatamodelError::new_model_validation_error(
            "Each model must have at most one id criteria. You can't have `@id` and `@@id` at the same time.",
            &model.name,
            ast_model.span,
        ));

        let has_single_field_id = model.singular_id_fields().next().is_some();
        let has_multi_field_id = !model.id_fields.is_empty();

        if model.singular_id_fields().count() > 1 {
            return multiple_single_field_id_error;
        }

        if has_single_field_id && has_multi_field_id {
            return multiple_id_criteria_error;
        }

        let loose_criterias = model.loose_unique_criterias();
        let suffix = if loose_criterias.is_empty() {
            "".to_string()
        } else {
            let criteria_descriptions: Vec<_> = loose_criterias
                .iter()
                .map(|criteria| {
                    let field_names: Vec<_> = criteria.fields.iter().map(|f| f.name.clone()).collect();
                    format!("- {}", field_names.join(", "))
                })
                .collect();
            format!(
                " The following unique criterias were not considered as they contain fields that are not required:\n{}",
                criteria_descriptions.join("\n")
            )
        };
        let missing_id_criteria_error = Err(DatamodelError::new_model_validation_error(
            &format!(
                "Each model must have at least one unique criteria that has only required fields. Either mark a single field with `@id`, `@unique` or add a multi field criterion with `@@id([])` or `@@unique([])` to the model.{suffix}",
                suffix = suffix
            ),
            &model.name,
            ast_model.span,
        ));

        if model.strict_unique_criterias().is_empty() {
            return missing_id_criteria_error;
        }

        Ok(())
    }

    fn validate_model_name(&self, ast_model: &ast::Model, model: &dml::Model) -> Result<(), DatamodelError> {
        let validator = super::reserved_model_names::TypeNameValidator::new();

        if validator.is_reserved(&model.name) {
            Err(DatamodelError::new_model_validation_error(
                &format!(
                    "The model name `{}` is invalid. It is a reserved name. Please change it. Read more at https://pris.ly/d/naming-models",
                    &model.name
                ),
                &model.name,
                ast_model.span,
            ))
        } else {
            Ok(())
        }
    }

    fn validate_enum_name(&self, ast_enum: &ast::Enum, dml_enum: &dml::Enum) -> Result<(), DatamodelError> {
        let validator = super::reserved_model_names::TypeNameValidator::new();

        if validator.is_reserved(&dml_enum.name) {
            Err(DatamodelError::new_enum_validation_error(
        &format!(
          "The enum name `{}` is invalid. It is a reserved name. Please change it. Read more at https://www.prisma.io/docs/reference/tools-and-interfaces/prisma-schema/data-model#naming-enums",
          &dml_enum.name
        ),
        &dml_enum.name,
        ast_enum.span,
      ))
        } else {
            Ok(())
        }
    }

    fn validate_field_connector_specific(&self, ast_model: &ast::Model, model: &dml::Model) -> Result<(), Diagnostics> {
        let mut diagnostics = Diagnostics::new();

        if let Some(source) = self.source {
            let connector = &source.active_connector;
            for field in model.fields.iter() {
                if let Err(err) = connector.validate_field(field) {
                    diagnostics.push_error(DatamodelError::new_connector_error(
                        &err.to_string(),
                        ast_model.find_field(&field.name()).span,
                    ));
                }
            }
        }

        if diagnostics.has_errors() {
            Err(diagnostics)
        } else {
            Ok(())
        }
    }

    /// Ensures that embedded types do not have back relations
    /// to their parent types.
    fn validate_embedded_types_have_no_back_relation(
        &self,
        ast_schema: &ast::SchemaAst,
        datamodel: &dml::Datamodel,
        model: &dml::Model,
    ) -> Result<(), DatamodelError> {
        if model.is_embedded {
            for field in model.relation_fields() {
                if !field.is_generated {
                    let rel_info = &field.relation_info;
                    // TODO: I am not sure if this check is d'accord with the query engine.
                    let related_field = datamodel.find_related_field_bang(&field);

                    if rel_info.to_fields.is_empty() && !related_field.is_generated {
                        // TODO: Refactor that out, it's way too much boilerplate.
                        return Err(DatamodelError::new_model_validation_error(
                            "Embedded models cannot have back relation fields.",
                            &model.name,
                            ast_schema.find_field(&model.name, &field.name).expect(STATE_ERROR).span,
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_base_fields_for_relation(
        &self,
        _datamodel: &dml::Datamodel,
        ast_model: &ast::Model,
        model: &dml::Model,
    ) -> Result<(), Diagnostics> {
        let mut errors = Diagnostics::new();

        for field in model.relation_fields() {
            let ast_field = ast_model.find_field(&field.name);

            let rel_info = &field.relation_info;
            let unknown_fields: Vec<String> = rel_info
                .fields
                .iter()
                .filter(|base_field| model.find_field(&base_field).is_none())
                .cloned()
                .collect();

            let referenced_relation_fields: Vec<String> = rel_info
                .fields
                .iter()
                .filter(|base_field| model.find_relation_field(&base_field).is_some())
                .cloned()
                .collect();

            let at_least_one_underlying_field_is_required = rel_info
                .fields
                .iter()
                .filter_map(|base_field| model.find_scalar_field(&base_field))
                .any(|f| f.is_required());

            let all_underlying_fields_are_optional = rel_info
                .fields
                .iter()
                .map(|base_field| match model.find_scalar_field(&base_field) {
                    Some(f) => f.is_optional(),
                    None => false,
                })
                .all(|x| x)
                && !rel_info.fields.is_empty(); // TODO: hack to maintain backwards compatibility for test schemas that don't specify fields yet

            if !unknown_fields.is_empty() {
                errors.push_error(DatamodelError::new_validation_error(
                        &format!("The argument fields must refer only to existing fields. The following fields do not exist in this model: {}", unknown_fields.join(", ")),
                        ast_field.span)
                    );
            }

            if !referenced_relation_fields.is_empty() {
                errors.push_error(DatamodelError::new_validation_error(
                        &format!("The argument fields must refer only to scalar fields. But it is referencing the following relation fields: {}", referenced_relation_fields.join(", ")),
                        ast_field.span)
                    );
            }

            if at_least_one_underlying_field_is_required && !field.is_required() {
                errors.push_error(DatamodelError::new_validation_error(
                        &format!(
                            "The relation field `{}` uses the scalar fields {}. At least one of those fields is required. Hence the relation field must be required as well.",
                            &field.name,
                            rel_info.fields.join(", ")
                        ),
                        ast_field.span)
                    );
            }

            if all_underlying_fields_are_optional && field.is_required() {
                errors.push_error(DatamodelError::new_validation_error(
                        &format!(
                            "The relation field `{}` uses the scalar fields {}. All those fields are optional. Hence the relation field must be optional as well.",
                            &field.name,
                            rel_info.fields.join(", ")
                        ),
                        ast_field.span)
                    );
            }
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(())
        }
    }

    fn validate_referenced_fields_for_relation(
        &self,
        datamodel: &dml::Datamodel,
        ast_model: &ast::Model,
        model: &dml::Model,
    ) -> Result<(), Diagnostics> {
        let mut errors = Diagnostics::new();

        for field in model.relation_fields() {
            let ast_field = ast_model.find_field(&field.name);

            let rel_info = &field.relation_info;
            let related_model = datamodel.find_model(&rel_info.to).expect(STATE_ERROR);
            let unknown_fields: Vec<String> = rel_info
                .to_fields
                .iter()
                .filter(|referenced_field| related_model.find_field(&referenced_field).is_none())
                .cloned()
                .collect();

            let referenced_relation_fields: Vec<String> = rel_info
                .to_fields
                .iter()
                .filter(|base_field| related_model.find_relation_field(&base_field).is_some())
                .cloned()
                .collect();

            let fields_with_wrong_type: Vec<DatamodelError> = rel_info.fields.iter().zip(rel_info.to_fields.iter())
                    .filter_map(|(base_field, referenced_field)| {
                        let base_field = model.find_field(&base_field)?;
                        let referenced_field = related_model.find_field(&referenced_field)?;

                        if !base_field.field_type().is_compatible_with(&referenced_field.field_type()) {
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
                        } else {
                            None
                        }
                    })
                    .collect();

            if !unknown_fields.is_empty() {
                errors.push_error(DatamodelError::new_validation_error(
                        &format!("The argument `references` must refer only to existing fields in the related model `{}`. The following fields do not exist in the related model: {}",
                                 &related_model.name,
                                 unknown_fields.join(", ")),
                        ast_field.span)
                    );
            }

            if !referenced_relation_fields.is_empty() {
                errors.push_error(DatamodelError::new_validation_error(
                        &format!("The argument `references` must refer only to scalar fields in the related model `{}`. But it is referencing the following relation fields: {}",
                                 &related_model.name,
                                 referenced_relation_fields.join(", ")),
                        ast_field.span)
                    );
            }

            if !rel_info.to_fields.is_empty() && !errors.has_errors() {
                // when we have other errors already don't push this error additionally
                let references_unique_criteria = related_model.loose_unique_criterias().iter().any(|criteria| {
                    let mut criteria_field_names: Vec<_> = criteria.fields.iter().map(|f| f.name.to_owned()).collect();
                    criteria_field_names.sort();

                    let mut to_fields_sorted = rel_info.to_fields.clone();
                    to_fields_sorted.sort();

                    criteria_field_names == to_fields_sorted
                });

                let references_singular_id_field = if rel_info.to_fields.len() == 1 {
                    let field_name = rel_info.to_fields.first().unwrap();
                    // the unwrap is safe. We error out earlier if an unknown field is referenced.
                    let referenced_field = related_model.find_scalar_field(&field_name).unwrap();
                    referenced_field.is_id
                } else {
                    false
                };
                let is_many_to_many = {
                    // Back relation fields have not been added yet. So we must calculate this on our own.
                    match datamodel.find_related_field(&field) {
                        Some(related_field) => field.is_list() && related_field.is_list(),
                        None => false,
                    }
                };

                let must_reference_unique_criteria = match self.source {
                    Some(source) => !source.combined_connector.supports_relations_over_non_unique_criteria(),
                    None => true,
                };

                if !references_unique_criteria && must_reference_unique_criteria {
                    errors.push_error(DatamodelError::new_validation_error(
                            &format!("The argument `references` must refer to a unique criteria in the related model `{}`. But it is referencing the following fields that are not a unique criteria: {}",
                                     &related_model.name,
                                     rel_info.to_fields.join(", ")),
                            ast_field.span)
                        );
                }

                let references_nullable_field = rel_info.to_fields.iter().any(|field_name| {
                    let referenced_field = related_model.find_scalar_field(&field_name).unwrap();
                    referenced_field.is_optional()
                });

                let must_not_reference_nullable_field = match self.source {
                    Some(source) => !source.combined_connector.supports_relations_over_nullable_field(),
                    None => false,
                };

                if references_nullable_field && must_not_reference_nullable_field {
                    errors.push_error(DatamodelError::new_validation_error(
                        &format!("The argument `references` must not refer to a nullable field in the related model `{}`. But it is referencing the following fields that are nullable: {}",
                                &related_model.name,
                                rel_info.to_fields.join(", ")),
                    ast_field.span)
                    );
                }

                // TODO: This error is only valid for connectors that don't support native many to manys.
                // We only render this error if there's a singular id field. Otherwise we render a better error in a different function.
                if is_many_to_many && !references_singular_id_field && related_model.has_single_id_field() {
                    errors.push_error(DatamodelError::new_validation_error(
                            &format!(
                                "Many to many relations must always reference the id field of the related model. Change the argument `references` to use the id field of the related model `{}`. But it is referencing the following fields that are not the id: {}",
                                &related_model.name,
                                rel_info.to_fields.join(", ")
                            ),
                            ast_field.span)
                        );
                }
            }

            if !rel_info.fields.is_empty()
                && !rel_info.to_fields.is_empty()
                && rel_info.fields.len() != rel_info.to_fields.len()
            {
                errors.push_error(DatamodelError::new_attribute_validation_error(
                    "You must specify the same number of fields in `fields` and `references`.",
                    RELATION_ATTRIBUTE_NAME,
                    ast_field.span,
                ));
            }

            if !fields_with_wrong_type.is_empty() && !errors.has_errors() {
                // don't output too much errors
                errors.append_error_vec(fields_with_wrong_type);
            }
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(())
        }
    }

    fn validate_relation_arguments_bla(
        &self,
        datamodel: &dml::Datamodel,
        ast_model: &ast::Model,
        model: &dml::Model,
    ) -> Diagnostics {
        let mut errors = Diagnostics::new();

        for field in model.relation_fields() {
            let field_span = ast_model
                .fields
                .iter()
                .find(|ast_field| ast_field.name.name == field.name)
                .map(|ast_field| ast_field.span)
                .unwrap_or_else(ast::Span::empty);

            let rel_info = &field.relation_info;
            let related_model = datamodel.find_model(&rel_info.to).expect(STATE_ERROR);
            let related_field = datamodel.find_related_field_bang(&field);
            let related_field_rel_info = &related_field.relation_info;

            // ONE TO MANY
            if field.is_singular() && related_field.is_list() {
                if rel_info.fields.is_empty() {
                    errors.push_error(DatamodelError::new_attribute_validation_error(
                        &format!(
                            "The relation field `{}` on Model `{}` must specify the `fields` argument in the {} attribute. {}",
                            &field.name, &model.name, RELATION_ATTRIBUTE_NAME_WITH_AT, PRISMA_FORMAT_HINT
                            ),
                        RELATION_ATTRIBUTE_NAME,
                        field_span,
                        ));
                }

                if rel_info.to_fields.is_empty() {
                    errors.push_error(DatamodelError::new_attribute_validation_error(
                        &format!(
                            "The relation field `{}` on Model `{}` must specify the `references` argument in the {} attribute.",
                            &field.name, &model.name, RELATION_ATTRIBUTE_NAME_WITH_AT
                            ),
                        RELATION_ATTRIBUTE_NAME,
                        field_span,
                        ));
                }
            }

            if field.is_list()
                && !related_field.is_list()
                && (!rel_info.fields.is_empty() || !rel_info.to_fields.is_empty())
            {
                errors.push_error(DatamodelError::new_attribute_validation_error(
                    &format!(
                        "The relation field `{}` on Model `{}` must not specify the `fields` or `references` argument in the {} attribute. You must only specify it on the opposite field `{}` on model `{}`.",
                        &field.name, &model.name, RELATION_ATTRIBUTE_NAME_WITH_AT, &related_field.name, &related_model.name
                            ),
                    RELATION_ATTRIBUTE_NAME,
                    field_span,
                        ));
            }

            // required ONE TO ONE SELF RELATION
            let is_self_relation = model.name == related_model.name;
            if is_self_relation && field.is_required() && related_field.is_required() {
                errors.push_error(DatamodelError::new_field_validation_error(
                        &format!(
                            "The relation fields `{}` and `{}` on Model `{}` are both required. This is not allowed for a self relation because it would not be possible to create a record.",
                            &field.name, &related_field.name, &model.name,
                        ),
                        &model.name,
                        &field.name,
                        field_span,
                    ));
            }

            // ONE TO ONE
            if field.is_singular() && related_field.is_singular() {
                if rel_info.fields.is_empty() && related_field_rel_info.fields.is_empty() {
                    errors.push_error(DatamodelError::new_attribute_validation_error(
                        &format!(
                            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` do not provide the `fields` argument in the {} attribute. You have to provide it on one of the two fields.",
                            &field.name, &model.name, &related_field.name, &related_model.name, RELATION_ATTRIBUTE_NAME_WITH_AT
                            ),
                        RELATION_ATTRIBUTE_NAME,
                        field_span,
                        ));
                }

                if rel_info.to_fields.is_empty() && related_field_rel_info.to_fields.is_empty() {
                    errors.push_error(DatamodelError::new_attribute_validation_error(
                        &format!(
                            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` do not provide the `references` argument in the {} attribute. You have to provide it on one of the two fields.",
                            &field.name, &model.name, &related_field.name, &related_model.name, RELATION_ATTRIBUTE_NAME_WITH_AT
                            ),
                        RELATION_ATTRIBUTE_NAME,
                        field_span,
                        ));
                }

                if !rel_info.to_fields.is_empty() && !related_field_rel_info.to_fields.is_empty() {
                    errors.push_error(DatamodelError::new_attribute_validation_error(
                        &format!(
                            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` both provide the `references` argument in the {} attribute. You have to provide it only on one of the two fields.",
                            &field.name, &model.name, &related_field.name, &related_model.name, RELATION_ATTRIBUTE_NAME_WITH_AT
                            ),
                        RELATION_ATTRIBUTE_NAME,
                        field_span,
                        ));
                }

                if !rel_info.fields.is_empty() && !related_field_rel_info.fields.is_empty() {
                    errors.push_error(DatamodelError::new_attribute_validation_error(
                        &format!(
                            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` both provide the `fields` argument in the {} attribute. You have to provide it only on one of the two fields.",
                            &field.name, &model.name, &related_field.name, &related_model.name, RELATION_ATTRIBUTE_NAME_WITH_AT
                            ),
                        RELATION_ATTRIBUTE_NAME,
                        field_span,
                        ));
                }

                if !errors.has_errors() {
                    if !rel_info.fields.is_empty() && !related_field_rel_info.to_fields.is_empty() {
                        errors.push_error(DatamodelError::new_attribute_validation_error(
                            &format!(
                                "The relation field `{}` on Model `{}` provides the `fields` argument in the {} attribute. And the related field `{}` on Model `{}` provides the `references` argument. You must provide both arguments on the same side.",
                                &field.name, &model.name, RELATION_ATTRIBUTE_NAME_WITH_AT, &related_field.name, &related_model.name,
                            ),
                            RELATION_ATTRIBUTE_NAME,
                            field_span,
                            ));
                    }

                    if !rel_info.to_fields.is_empty() && !related_field_rel_info.fields.is_empty() {
                        errors.push_error(DatamodelError::new_attribute_validation_error(
                            &format!(
                                "The relation field `{}` on Model `{}` provides the `references` argument in the {} attribute. And the related field `{}` on Model `{}` provides the `fields` argument. You must provide both arguments on the same side.",
                                &field.name, &model.name, RELATION_ATTRIBUTE_NAME_WITH_AT, &related_field.name, &related_model.name,
                                ),
                            RELATION_ATTRIBUTE_NAME,
                            field_span,
                            ));
                    }
                }
            }

            // MANY TO MANY
            if field.is_list() && related_field.is_list() && !related_model.has_single_id_field() {
                errors.push_error(DatamodelError::new_field_validation_error(
                            &format!(
                                "The relation field `{}` on Model `{}` references `{}` which does not have an `@id` field. Models without `@id` can not be part of a many to many relation. Use an explicit intermediate Model to represent this relationship.",
                                &field.name,
                                &model.name,
                                &related_model.name,
                            ),
                            &model.name,
                            &field.name,
                            field_span,
                        ));
            }
        }

        errors
    }

    /// Elegantly checks if any relations in the model are ambigious.
    fn validate_relations_not_ambiguous(
        &self,
        ast_schema: &ast::SchemaAst,
        model: &dml::Model,
    ) -> Result<(), DatamodelError> {
        for field_a in model.relation_fields() {
            for field_b in model.relation_fields() {
                if field_a != field_b {
                    let rel_a = &field_a.relation_info;
                    let rel_b = &field_b.relation_info;
                    if rel_a.to != model.name && rel_b.to != model.name {
                        // Not a self relation
                        // but pointing to the same foreign model,
                        // and also no names set.
                        if rel_a.to == rel_b.to && rel_a.name == rel_b.name {
                            if rel_a.name == "" {
                                // unnamed relation
                                return Err(DatamodelError::new_model_validation_error(
                                            &format!(
                                                "Ambiguous relation detected. The fields `{}` and `{}` in model `{}` both refer to `{}`. Please provide different relation names for them by adding `@relation(<name>).",
                                                &field_a.name,
                                                &field_b.name,
                                                &model.name,
                                                &rel_a.to
                                            ),
                                            &model.name,
                                            ast_schema
                                                .find_field(&model.name, &field_a.name)
                                                .expect(STATE_ERROR)
                                                .span,
                                        ));
                            } else {
                                // explicitly named relation
                                return Err(DatamodelError::new_model_validation_error(
                                            &format!(
                                                "Wrongly named relation detected. The fields `{}` and `{}` in model `{}` both use the same relation name. Please provide different relation names for them through `@relation(<name>).",
                                                &field_a.name,
                                                &field_b.name,
                                                &model.name,
                                            ),
                                            &model.name,
                                            ast_schema
                                                .find_field(&model.name, &field_a.name)
                                                .expect(STATE_ERROR)
                                                .span,
                                        ));
                            }
                        }
                    } else if rel_a.to == model.name && rel_b.to == model.name {
                        // This is a self-relation with at least two fields.

                        // Named self relations are ambiguous when they involve more than two fields.
                        for field_c in model.relation_fields() {
                            if field_a != field_c && field_b != field_c {
                                let rel_c = &field_c.relation_info;
                                if rel_c.to == model.name && rel_a.name == rel_b.name && rel_a.name == rel_c.name {
                                    if rel_a.name == "" {
                                        // unnamed relation
                                        return Err(DatamodelError::new_model_validation_error(
                                                        &format!(
                                                            "Unnamed self relation detected. The fields `{}`, `{}` and `{}` in model `{}` have no relation name. Please provide a relation name for one of them by adding `@relation(<name>).",
                                                            &field_a.name,
                                                            &field_b.name,
                                                            &field_c.name,
                                                            &model.name
                                                        ),
                                                        &model.name,
                                                        ast_schema
                                                            .find_field(&model.name, &field_a.name)
                                                            .expect(STATE_ERROR)
                                                            .span,
                                                    ));
                                    } else {
                                        return Err(DatamodelError::new_model_validation_error(
                                                        &format!(
                                                        "Wrongly named self relation detected. The fields `{}`, `{}` and `{}` in model `{}` have the same relation name. At most two relation fields can belong to the same relation and therefore have the same name. Please assign a different relation name to one of them.",
                                                            &field_a.name,
                                                            &field_b.name,
                                                            &field_c.name,
                                                            &model.name
                                                        ),
                                                        &model.name,
                                                        ast_schema
                                                            .find_field(&model.name, &field_a.name)
                                                            .expect(STATE_ERROR)
                                                            .span,
                                                    ));
                                    }
                                }
                            }
                        }

                        // Ambiguous unnamed self relation: two fields are enough.
                        if rel_a.name.is_empty() && rel_b.name.is_empty() {
                            // A self relation, but there are at least two fields without a name.
                            return Err(DatamodelError::new_model_validation_error(
                                        &format!(
                                            "Ambiguous self relation detected. The fields `{}` and `{}` in model `{}` both refer to `{}`. If they are part of the same relation add the same relation name for them with `@relation(<name>)`.",
                                            &field_a.name,
                                            &field_b.name,
                                            &model.name,
                                            &rel_a.to
                                        ),
                                        &model.name,
                                        ast_schema
                                            .find_field(&model.name, &field_a.name)
                                            .expect(STATE_ERROR)
                                            .span,
                                    ));
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
