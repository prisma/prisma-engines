use super::db::ParserDatabase;
use crate::ast::{Model, Span};
use crate::common::datamodel_context::DatamodelContext;
use crate::{
    ast,
    diagnostics::{DatamodelError, Diagnostics},
    dml,
    walkers::ModelWalker,
    DefaultValue, FieldType, IndexType,
};
use crate::{ast::WithAttributes, walkers::walk_models};
use itertools::Itertools;
use prisma_value::PrismaValue;
use std::collections::{HashMap, HashSet};

/// Helper for validating a datamodel.
///
/// When validating, we check if the datamodel is valid, and generate errors otherwise.
pub struct Validator<'a> {
    context: &'a DatamodelContext,
}

/// State error message. Seeing this error means something went really wrong internally. It's the datamodel equivalent of a bluescreen.
const STATE_ERROR: &str = "Failed lookup of model, field or optional property during internal processing. This means that the internal representation was mutated incorrectly.";
const RELATION_ATTRIBUTE_NAME: &str = "relation";
const RELATION_ATTRIBUTE_NAME_WITH_AT: &str = "@relation";
const PRISMA_FORMAT_HINT: &str = "You can run `prisma format` to fix this automatically.";

impl<'a> Validator<'a> {
    /// Creates a new instance, with all builtin attributes registered.
    pub(crate) fn new(context: &'a DatamodelContext) -> Validator<'a> {
        Self { context }
    }

    pub(crate) fn validate(&self, db: &ParserDatabase<'_>, schema: &mut dml::Datamodel) -> Result<(), Diagnostics> {
        let mut all_errors = Diagnostics::new();
        let ast_schema = db.ast();

        //todo use this pattern everywhere instead of the if let result dance?
        self.validate_names(&mut all_errors, db);

        if let Err(errs) = self.validate_names_for_indexes(ast_schema, schema) {
            all_errors.extend(errs);
        }

        // Model level validations.
        for model in schema.models() {
            // Having a separate error collection allows checking whether any error has occurred for a model.
            let mut errors_for_model = Diagnostics::new();
            let ast_model = ast_schema.find_model(&model.name).expect(STATE_ERROR);

            self.validate_optional_id_only_on_ignored_models(&mut errors_for_model, &model, ast_model);

            self.validate_name_collisions_with_compound_client_names(&mut errors_for_model, model, ast_model);

            self.validate_constraint_name_lengths(&mut errors_for_model, model, ast_model);

            if let Err(err) = self.validate_model_has_strict_unique_criteria(ast_model, model) {
                errors_for_model.push_error(err);
            }
            let ast_model = ast_schema.find_model(&model.name).expect(STATE_ERROR);

            if let Err(err) = self.validate_model_compound_ids(ast_model, model) {
                errors_for_model.push_error(err);
            }

            if let Err(err) = self.validate_model_name(ast_model, model) {
                errors_for_model.push_error(err);
            }

            if let Err(err) = self.validate_relations_not_ambiguous(ast_schema, model) {
                errors_for_model.push_error(err);
            }

            if let Err(the_errors) = self.validate_field_arities(ast_model, model) {
                errors_for_model.extend(the_errors);
            }

            if let Err(the_errors) = self.validate_field_types(ast_model, model) {
                errors_for_model.extend(the_errors);
            }

            if let Err(the_errors) = self.validate_field_connector_specific(ast_model, model) {
                errors_for_model.extend(the_errors);
            }

            if let Err(the_errors) = self.validate_model_connector_specific(ast_model, model) {
                errors_for_model.extend(the_errors);
            }

            if let Err(the_errors) = self.validate_enum_default_values(schema, ast_model, model) {
                errors_for_model.extend(the_errors);
            }

            if let Err(the_errors) = self.validate_auto_increment(ast_model, model) {
                errors_for_model.extend(the_errors);
            }

            if let Err(the_errors) = self.validate_base_fields_for_relation(ast_model, model) {
                errors_for_model.extend(the_errors);
            }

            if let Err(the_errors) = self.validate_referenced_fields_for_relation(schema, ast_model, model) {
                errors_for_model.extend(the_errors);
            }

            all_errors.extend(errors_for_model);
        }

        validate_name_collisions_with_map(schema, ast_schema, &mut all_errors);

        // Enum level validations.
        for declared_enum in schema.enums() {
            let mut errors_for_enum = Diagnostics::new();
            if let Err(err) =
                self.validate_enum_name(db.get_enum(&declared_enum.name).expect(STATE_ERROR), declared_enum)
            {
                errors_for_enum.push_error(err);
            }

            all_errors.extend(errors_for_enum);
        }

        all_errors.make_result()
    }

    pub fn validate_optional_id_only_on_ignored_models(
        &self,
        errors_for_model: &mut Diagnostics,
        model: &dml::Model,
        ast_model: &Model,
    ) {
        //todo align error handling and unwrapping with the rest
        if let Some(sf) = &model.scalar_fields().find(|f| f.is_id() && !f.is_required()) {
            if !&model.is_ignored {
                let span = ast_model
                    .find_field(&sf.name)
                    .map(|f| f.attributes.iter().find(|att| att.name.name == "id").unwrap().span)
                    .unwrap_or_else(ast::Span::empty);

                errors_for_model.push_error(DatamodelError::new_attribute_validation_error(
                    "Fields that are marked as id must be required.",
                    "id",
                    span,
                ));
            }
        }
    }

    pub fn validate_name_collisions_with_compound_client_names(
        &self,
        errors_for_model: &mut Diagnostics,
        model: &dml::Model,
        ast_model: &Model,
    ) {
        let model_span = ast_model.span;

        if let Some(pk) = &model.primary_key {
            if let Some(name) = &pk.name_in_client {
                for field in model.scalar_fields() {
                    custom_name_reused(errors_for_model, name, &field.name, &model.name, "@@id", model_span)
                }
                for field in model.relation_fields() {
                    custom_name_reused(errors_for_model, name, &field.name, &model.name, "@@id", model_span)
                }
            }
        }

        for index in model.indices.iter().filter(|i| i.is_unique() && i.fields.len() > 1) {
            if let Some(name) = &index.name_in_client {
                for field in model.scalar_fields() {
                    custom_name_reused(errors_for_model, name, &field.name, &model.name, "@@unique", model_span)
                }
                for field in model.relation_fields() {
                    custom_name_reused(errors_for_model, name, &field.name, &model.name, "@@unique", model_span)
                }
            }
        }
    }

    pub fn validate_constraint_name_lengths(
        &self,
        errors_for_model: &mut Diagnostics,
        model: &dml::Model,
        ast_model: &Model,
    ) {
        let length_limit = self.context.connector.constraint_name_length();
        let model_span = ast_model.span;

        if let Some(pk) = &model.primary_key {
            if let Some(name) = &pk.name_in_db {
                constraint_name_length_violated(errors_for_model, name, &model.name, "id", length_limit, model_span)
            }
        }

        for index in &model.indices {
            let attribute = match index.tpe {
                IndexType::Unique => "@unique",
                IndexType::Normal => "@@index",
            };
            constraint_name_length_violated(
                errors_for_model,
                &index.name_in_db,
                &model.name,
                attribute,
                length_limit,
                model_span,
            );
        }

        for rf in model.relation_fields() {
            if let Some(fk_name) = &rf.relation_info.fk_name {
                constraint_name_length_violated(
                    errors_for_model,
                    fk_name,
                    &model.name,
                    "@relation",
                    length_limit,
                    model_span,
                );
            }
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
                let new_errors = self.validate_relation_arguments_bla(
                    schema,
                    ast_schema.find_model(&model.name).expect(STATE_ERROR),
                    model,
                );
                errors_for_model.extend(new_errors);
            }

            all_errors.extend(errors_for_model);
        }

        all_errors.make_result()
    }

    fn validate_names(&self, errors: &mut Diagnostics, db: &ParserDatabase<'_>) {
        for model in db.ast().models() {
            errors.push_opt_error(model.name.validate("Model").err());
            errors.extend(model.validate_attributes());

            for field in model.fields.iter() {
                errors.push_opt_error(field.name.validate("Field").err());
                errors.extend(field.validate_attributes());
            }
        }

        for (_, enum_decl) in db.iter_enums() {
            errors.push_opt_error(enum_decl.name.validate("Enum").err());
            errors.extend(enum_decl.validate_attributes());

            for enum_value in enum_decl.values.iter() {
                errors.push_opt_error(enum_value.name.validate("Enum Value").err());
                errors.extend(enum_value.validate_attributes());
            }
        }
    }

    fn validate_names_for_indexes(
        &self,
        ast_schema: &ast::SchemaAst,
        schema: &dml::Datamodel,
    ) -> Result<(), Diagnostics> {
        let mut errors = Diagnostics::new();
        let mut index_names = HashSet::new();

        if !self.context.connector.supports_multiple_indexes_with_same_name() {
            for model in schema.models() {
                if let Some(ast_model) = ast_schema.find_model(&model.name) {
                    for index in &model.indices {
                        if index_names.contains(&index.name_in_db) {
                            let ast_index = ast_model
                                .attributes
                                .iter()
                                .find(|attribute| attribute.is_index())
                                .unwrap();

                            errors.push_error(DatamodelError::new_multiple_indexes_with_same_name_are_not_supported(
                                &index.name_in_db,
                                ast_index.span,
                            ));
                        }
                        index_names.insert(&index.name_in_db);
                    }
                }
            }
        }

        errors.make_result()
    }

    fn validate_field_arities(&self, ast_model: &ast::Model, model: &dml::Model) -> Result<(), Diagnostics> {
        let mut errors = Diagnostics::new();

        if !self.context.connector.supports_scalar_lists() {
            for field in model.scalar_fields().filter(|f| f.is_list()) {
                errors.push_error(DatamodelError::new_scalar_list_fields_are_not_supported(
                    &model.name,
                    &field.name,
                    ast_model.find_field_bang(&field.name).span,
                ));
            }
        }

        errors.make_result()
    }

    fn validate_field_types(&self, ast_model: &ast::Model, model: &dml::Model) -> Result<(), Diagnostics> {
        let mut errors = Diagnostics::new();

        if !self.context.connector.supports_json() {
            for field in model
                .scalar_fields()
                .filter(|f| matches!(f.field_type.scalar_type(), Some(dml::ScalarType::Json)))
            {
                errors.push_error(DatamodelError::new_field_validation_error(
                        &format!("Field `{}` in model `{}` can't be of type Json. The current connector does not support the Json type.", &field.name, &model.name),
                        &model.name,
                        &field.name,
                        ast_model.find_field_bang(&field.name).span,
                    ));
            }
        }

        errors.make_result()
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
                                ast_model.find_field_bang(&field.name).span,
                            ))
                        }
                    }
                }
            }
        }

        errors.make_result()
    }

    fn validate_auto_increment(&self, ast_model: &ast::Model, model: &dml::Model) -> Result<(), Diagnostics> {
        let mut errors = Diagnostics::new();

        let autoinc_fields = model.auto_increment_fields().collect_vec();

        // First check if the provider supports autoincrement at all, if yes, proceed with the detailed checks.
        if !autoinc_fields.is_empty() && !self.context.connector.supports_auto_increment() {
            for field in &autoinc_fields {
                let ast_field = ast_model.find_field_bang(&field.name);

                // Add an error for all autoincrement fields on the model.
                errors.push_error(DatamodelError::new_attribute_validation_error(
                    &"The `autoincrement()` default value is used with a datasource that does not support it."
                        .to_string(),
                    "default",
                    ast_field.span,
                ));
            }
        }

        if !self.context.connector.supports_multiple_auto_increment() && model.auto_increment_fields().count() > 1 {
            errors.push_error(DatamodelError::new_attribute_validation_error(
                    &"The `autoincrement()` default value is used multiple times on this model even though the underlying datasource only supports one instance per table.".to_string(),
                    "default",
                    ast_model.span,
                ))
        }

        // go over all fields
        for field in &autoinc_fields {
            let ast_field = ast_model.find_field_bang(&field.name);

            if !field.is_id() && !self.context.connector.supports_non_id_auto_increment() {
                errors.push_error(DatamodelError::new_attribute_validation_error(
                            &"The `autoincrement()` default value is used on a non-id field even though the datasource does not support this.".to_string(),
                            "default",
                            ast_field.span,
                        ))
            }

            if !model.field_is_indexed(&field.name) && !self.context.connector.supports_non_indexed_auto_increment() {
                errors.push_error(DatamodelError::new_attribute_validation_error(
                            &"The `autoincrement()` default value is used on a non-indexed field even though the datasource does not support this.".to_string(),
                            "default",
                            ast_field.span,
                        ))
            }
        }

        errors.make_result()
    }

    fn validate_model_has_strict_unique_criteria(
        &self,
        ast_model: &ast::Model,
        model: &dml::Model,
    ) -> Result<(), DatamodelError> {
        if model.singular_id_fields().count() > 1 {
            return Err(DatamodelError::new_model_validation_error(
                "At most one field must be marked as the id field with the `@id` attribute.",
                &model.name,
                ast_model.span,
            ));
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

        if model.strict_unique_criterias_disregarding_unsupported().is_empty() && !model.is_ignored {
            return Err(DatamodelError::new_model_validation_error(
                &format!(
                    "Each model must have at least one unique criteria that has only required fields. Either mark a single field with `@id`, `@unique` or add a multi field criterion with `@@id([])` or `@@unique([])` to the model.{suffix}",
                    suffix = suffix
                ),
                &model.name,
                ast_model.span,
            ));
        }

        Ok(())
    }

    fn validate_model_compound_ids(&self, ast_model: &ast::Model, model: &dml::Model) -> Result<(), DatamodelError> {
        if model.has_compound_id() && !self.context.connector.supports_compound_ids() {
            let ast_attr = ast_model
                .attributes()
                .iter()
                .find(|attr| &attr.name.name == "id")
                .unwrap();

            Err(DatamodelError::new_model_validation_error(
                "The current connector does not support compound ids.",
                &model.name,
                ast_attr.span,
            ))
        } else {
            Ok(())
        }
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

        for field in model.fields.iter() {
            if let Err(err) = self.context.connector.validate_field(field) {
                diagnostics.push_error(DatamodelError::new_connector_error(
                    &err.to_string(),
                    ast_model.find_field_bang(&field.name()).span,
                ));
            }
        }

        diagnostics.make_result()
    }

    fn validate_model_connector_specific(&self, ast_model: &ast::Model, model: &dml::Model) -> Result<(), Diagnostics> {
        let mut diagnostics = Diagnostics::new();

        if let Err(err) = self.context.connector.validate_model(model) {
            diagnostics.push_error(DatamodelError::new_connector_error(&err.to_string(), ast_model.span))
        }

        diagnostics.make_result()
    }

    fn validate_base_fields_for_relation(&self, ast_model: &ast::Model, model: &dml::Model) -> Result<(), Diagnostics> {
        let mut errors = Diagnostics::new();

        for field in model.relation_fields() {
            let ast_field = ast_model.find_field_bang(&field.name);

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

            let at_least_one_underlying_field_is_optional = rel_info
                .fields
                .iter()
                .filter_map(|base_field| model.find_scalar_field(&base_field))
                .any(|f| f.is_optional());

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

            if at_least_one_underlying_field_is_optional && field.is_required() {
                errors.push_error(DatamodelError::new_validation_error(
                        &format!(
                            "The relation field `{}` uses the scalar fields {}. At least one of those fields is optional. Hence the relation field must be optional as well.",
                            &field.name,
                            rel_info.fields.join(", ")
                        ),
                        ast_field.span)
                    );
            }
        }

        errors.make_result()
    }

    fn validate_referenced_fields_for_relation(
        &self,
        datamodel: &dml::Datamodel,
        ast_model: &ast::Model,
        model: &dml::Model,
    ) -> Result<(), Diagnostics> {
        let mut errors = Diagnostics::new();

        for field in model.relation_fields() {
            let ast_field = ast_model.find_field_bang(&field.name);

            let rel_info = &field.relation_info;
            let related_model = datamodel.find_model(&rel_info.to).expect(STATE_ERROR);

            let unknown_fields: Vec<String> = rel_info
                .references
                .iter()
                .filter(|referenced_field| related_model.find_field(&referenced_field).is_none())
                .cloned()
                .collect();

            let referenced_relation_fields: Vec<String> = rel_info
                .references
                .iter()
                .filter(|base_field| related_model.find_relation_field(&base_field).is_some())
                .cloned()
                .collect();

            let fields_with_wrong_type: Vec<DatamodelError> = rel_info.fields.iter().zip(rel_info.references.iter())
                    .filter_map(|(base_field, referenced_field)| {
                        let base_field = model.find_field(&base_field)?;
                        let referenced_field = related_model.find_field(&referenced_field)?;

                        if base_field.field_type().is_compatible_with(&referenced_field.field_type()) {
                            return None
                        }

                        // Try harder to see if the final type is not the same.
                        // This check needs the connector, so it can't be in the dml
                        // crate.
                        if !self.context.connector.is_empty_default() {
                            let base_native_type = base_field.field_type().as_native_type().map(|(scalar, native)| (*scalar, native.serialized_native_type.clone())).or_else(|| -> Option<_> {
                                let field_type = base_field.field_type();
                                let scalar_type = field_type.as_base()?;

                                Some((*scalar_type, self.context.connector.default_native_type_for_scalar_type(scalar_type)))
                            });

                            let referenced_native_type = referenced_field.field_type().as_native_type().map(|(scalar, native)| (*scalar, native.serialized_native_type.clone())).or_else(|| -> Option<_> {
                                let field_type = referenced_field.field_type();
                                let scalar_type = field_type.as_base()?;

                                Some((*scalar_type, self.context.connector.default_native_type_for_scalar_type(scalar_type)))
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

            if !rel_info.references.is_empty() && !errors.has_errors() {
                let strict_relation_field_order = self.context.connector.allows_relation_fields_in_arbitrary_order();

                // when we have other errors already don't push this error additionally
                let references_unique_criteria = related_model.loose_unique_criterias().iter().any(|criteria| {
                    let mut criteria_field_names: Vec<_> = criteria.fields.iter().map(|f| f.name.to_owned()).collect();
                    criteria_field_names.sort();

                    let mut references_sorted = rel_info.references.clone();
                    references_sorted.sort();

                    criteria_field_names == references_sorted
                });

                let reference_order_correct = if strict_relation_field_order && rel_info.references.len() > 1 {
                    related_model.loose_unique_criterias().iter().any(|criteria| {
                        let criteria_fields = criteria.fields.iter().map(|f| f.name.as_str());
                        let references = rel_info.references.iter().map(|f| f.as_str());

                        let same_length = criteria_fields.len() == references.len();
                        let same_order = criteria_fields.zip(references).all(|(a, b)| a == b);

                        same_length && same_order
                    })
                } else {
                    true
                };

                let references_singular_id_field = if rel_info.references.len() == 1 {
                    let field_name = rel_info.references.first().unwrap();
                    // the unwrap is safe. We error out earlier if an unknown field is referenced.
                    let referenced_field = related_model.find_scalar_field(&field_name).unwrap();
                    referenced_field.is_id()
                } else {
                    false
                };

                let is_many_to_many = {
                    // Back relation fields have not been added yet. So we must calculate this on our own.
                    match datamodel.find_related_field(&field) {
                        Some((_, related_field)) => field.is_list() && related_field.is_list(),
                        None => false,
                    }
                };

                let must_reference_unique_criteria =
                    self.context.connector.supports_relations_over_non_unique_criteria();

                if !references_unique_criteria && must_reference_unique_criteria {
                    errors.push_error(DatamodelError::new_validation_error(
                            &format!("The argument `references` must refer to a unique criteria in the related model `{}`. But it is referencing the following fields that are not a unique criteria: {}",
                                     &related_model.name,
                                     rel_info.references.join(", ")),
                            ast_field.span)
                        );
                } else if !reference_order_correct {
                    errors.push_error(DatamodelError::new_validation_error(
                        &format!("The argument `references` must refer to a unique criteria in the related model `{}` using the same order of fields. Please check the ordering in the following fields: `{}`.",
                                 &related_model.name,
                                 rel_info.references.join(", ")),
                        ast_field.span)
                    );
                }

                // TODO: This error is only valid for connectors that don't support native many to manys.
                // We only render this error if there's a singular id field. Otherwise we render a better error in a different function.
                if is_many_to_many && !references_singular_id_field && related_model.has_singular_id() {
                    errors.push_error(DatamodelError::new_validation_error(
                            &format!(
                                "Many to many relations must always reference the id field of the related model. Change the argument `references` to use the id field of the related model `{}`. But it is referencing the following fields that are not the id: {}",
                                &related_model.name,
                                rel_info.references.join(", ")
                            ),
                            ast_field.span)
                        );
                }
            }

            if !rel_info.fields.is_empty()
                && !rel_info.references.is_empty()
                && rel_info.fields.len() != rel_info.references.len()
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

        errors.make_result()
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
            if let Some((_rel_field_idx, related_field)) = datamodel.find_related_field(&field) {
                let related_field_rel_info = &related_field.relation_info;

                if related_model.is_ignored && !field.is_ignored && !model.is_ignored {
                    errors.push_error(DatamodelError::new_attribute_validation_error(
                    &format!(
                        "The relation field `{}` on Model `{}` must specify the `@ignore` attribute, because the model {} it is pointing to is marked ignored.",
                        &field.name, &model.name, &related_model.name
                    ),
                    "ignore",
                    field_span,
                ));
                }

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

                    if rel_info.references.is_empty() {
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
                    && (!rel_info.fields.is_empty() || !rel_info.references.is_empty())
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

                    if rel_info.references.is_empty() && related_field_rel_info.references.is_empty() {
                        errors.push_error(DatamodelError::new_attribute_validation_error(
                        &format!(
                            "The relation fields `{}` on Model `{}` and `{}` on Model `{}` do not provide the `references` argument in the {} attribute. You have to provide it on one of the two fields.",
                            &field.name, &model.name, &related_field.name, &related_model.name, RELATION_ATTRIBUTE_NAME_WITH_AT
                            ),
                        RELATION_ATTRIBUTE_NAME,
                        field_span,
                        ));
                    }

                    if !rel_info.references.is_empty() && !related_field_rel_info.references.is_empty() {
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
                        if !rel_info.fields.is_empty() && !related_field_rel_info.references.is_empty() {
                            errors.push_error(DatamodelError::new_attribute_validation_error(
                            &format!(
                                "The relation field `{}` on Model `{}` provides the `fields` argument in the {} attribute. And the related field `{}` on Model `{}` provides the `references` argument. You must provide both arguments on the same side.",
                                &field.name, &model.name, RELATION_ATTRIBUTE_NAME_WITH_AT, &related_field.name, &related_model.name,
                            ),
                            RELATION_ATTRIBUTE_NAME,
                            field_span,
                            ));
                        }

                        if !rel_info.references.is_empty() && !related_field_rel_info.fields.is_empty() {
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

                    if !errors.has_errors() && field.is_required() && !related_field_rel_info.references.is_empty() {
                        errors.push_error(DatamodelError::new_attribute_validation_error(
                            &format!(
                                "The relation field `{}` on Model `{}` is required. This is no longer valid because it's not possible to enforce this constraint on the database level. Please change the field type from `{}` to `{}?` to fix this.",
                                &field.name, &model.name, &related_model.name, &related_model.name,
                            ),
                            RELATION_ATTRIBUTE_NAME,
                            field_span,
                        ));
                    }
                }

                // MANY TO MANY
                if field.is_list() && related_field.is_list() && !related_model.has_singular_id() {
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
            } else {
                errors.push_error(DatamodelError::new_field_validation_error(
                    &format!(
                        "The relation field `{}` on Model `{}` is missing an opposite relation field on the model `{}`. Either run `prisma format` or add it manually.",
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
                            if rel_a.name.is_empty() {
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
                                    if rel_a.name.is_empty() {
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

fn validate_name_collisions_with_map(schema: &dml::Datamodel, ast: &ast::SchemaAst, diagnostics: &mut Diagnostics) {
    let mut used_model_names: HashMap<&str, ModelWalker<'_>> = HashMap::with_capacity(schema.models.len());
    let mut used_field_names: HashSet<&str> = HashSet::with_capacity(4);

    for model in walk_models(schema) {
        for field in model.scalar_fields() {
            if !used_field_names.insert(field.db_name()) {
                diagnostics.push_error(DatamodelError::new_duplicate_field_error(
                    model.name(),
                    field.name(),
                    ast.find_model(model.name()).unwrap().find_field_bang(field.name()).span,
                ));
            }
        }

        used_field_names.clear();

        if let Some(existing_model) = used_model_names.insert(model.database_name(), model) {
            diagnostics.push_error(DatamodelError::new_duplicate_model_database_name_error(
                model.database_name().into(),
                existing_model.name().into(),
                ast.find_model(model.name()).unwrap().span,
            ));
        }
    }
}

fn custom_name_reused(
    diagnostics: &mut Diagnostics,
    custom_name: &str,
    field_name: &str,
    model_name: &str,
    attribute: &str,
    span: Span,
) {
    if field_name == custom_name {
        diagnostics.push_error(DatamodelError::new_model_validation_error(
            &format!("The custom name specified for the `{}` attribute is already used as a name for a field. Please choose a different name.", attribute),
            model_name,
            span,
        ));
    }
}

fn constraint_name_length_violated(
    diagnostics: &mut Diagnostics,
    constraint_name: &str,
    model_name: &str,
    attribute: &str,
    length_limit: usize,
    span: Span,
) {
    if constraint_name.len() > length_limit {
        diagnostics.push_error(DatamodelError::new_model_validation_error(
            &format!("The name specified for the `{}` constraint `{}` is too long for your chosen provider. The maximum allowed length is {} bytes.", attribute, constraint_name, length_limit),
            model_name,
            span,
        ));
    }
}
