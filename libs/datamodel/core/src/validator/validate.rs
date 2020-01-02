use crate::{
    ast, configuration, dml,
    error::{DatamodelError, ErrorCollection},
    FieldArity,
};

/// Helper for validating a datamodel.
///
/// When validating, we check if the datamodel is valid, and generate errors otherwise.
pub struct Validator<'a> {
    source: Option<&'a Box<dyn configuration::Source + Send + Sync>>,
}

/// State error message. Seeing this error means something went really wrong internally. It's the datamodel equivalent of a bluescreen.
const STATE_ERROR: &str = "Failed lookup of model, field or optional property during internal processing. This means that the internal representation was mutated incorrectly.";

impl<'a> Validator<'a> {
    /// Creates a new instance, with all builtin directives registered.
    pub fn new(source: Option<&'a Box<dyn configuration::Source + Send + Sync>>) -> Validator {
        Self { source }
    }

    pub fn validate(&self, ast_schema: &ast::SchemaAst, schema: &mut dml::Datamodel) -> Result<(), ErrorCollection> {
        let mut errors = ErrorCollection::new();

        // Model level validations.
        for model in schema.models() {
            if let Err(err) = self.validate_model_has_id(ast_schema.find_model(&model.name).expect(STATE_ERROR), model)
            {
                errors.push(err);
            }
            if let Err(err) = self.validate_id_fields_valid(ast_schema, model) {
                errors.push(err);
            }
            if let Err(err) = self.validate_relations_not_ambiguous(ast_schema, model) {
                errors.push(err);
            }
            if let Err(err) = self.validate_embedded_types_have_no_back_relation(ast_schema, schema, model) {
                errors.push(err);
            }

            if let Err(ref mut the_errors) =
                self.validate_field_arities(ast_schema.find_model(&model.name).expect(STATE_ERROR), model)
            {
                errors.append(the_errors);
            }
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(())
        }
    }

    fn validate_field_arities(&self, ast_model: &ast::Model, model: &dml::Model) -> Result<(), ErrorCollection> {
        let mut errors = ErrorCollection::new();

        // TODO: this is really ugly
        let scalar_lists_are_supported = match self.source {
            Some(source) => source.connector().supports_scalar_lists(),
            None => false,
        };

        for field in model.fields() {
            if field.arity == FieldArity::List && !scalar_lists_are_supported && !field.field_type.is_relation() {
                let ast_field = ast_model
                    .fields
                    .iter()
                    .find(|ast_field| ast_field.name.name == field.name)
                    .unwrap();

                errors.push(DatamodelError::new_scalar_list_fields_are_not_supported(
                    &model.name,
                    &field.name,
                    ast_field.span,
                ));
            }
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(())
        }
    }

    fn validate_model_has_id(&self, ast_model: &ast::Model, model: &dml::Model) -> Result<(), DatamodelError> {
        if model.is_relation_model() {
            return Ok(());
            // Extempt from the id rule, we have an relation table.
        }

        let multiple_single_field_id_error = Err(DatamodelError::new_model_validation_error(
            "At most one field must be marked as the id field with the `@id` directive.",
            &model.name,
            ast_model.span,
        ));

        let multiple_id_criteria_error = Err(DatamodelError::new_model_validation_error(
            "Each model must have exactly one id criteria. Either mark a single field with `@id` or add a multi field id criterion with `@@id([])` to the model.",
            &model.name,
            ast_model.span,
        ));

        match (model.singular_id_fields().count(), model.id_fields.is_empty()) {
            (c, _) if c > 1 => multiple_single_field_id_error,
            (0, true) => multiple_id_criteria_error,
            (1, false) => multiple_id_criteria_error,
            (1, true) | (0, false) => Ok(()),
            (_, _) => unreachable!(), // the compiler does not check the first if guard
        }
    }

    fn validate_id_fields_valid(&self, ast_schema: &ast::SchemaAst, model: &dml::Model) -> Result<(), DatamodelError> {
        for id_field in model.singular_id_fields() {
            let is_valid = match (&id_field.default_value, &id_field.field_type, &id_field.arity) {
                (
                    Some(dml::ScalarValue::Expression(name, return_type, args)),
                    dml::FieldType::Base(dml::ScalarType::String),
                    dml::FieldArity::Required,
                ) => {
                    let name_eq = name == "cuid" || name == "uuid";
                    let type_eq = return_type == &dml::ScalarType::String;
                    let args_eq = args.is_empty();

                    name_eq && type_eq && args_eq
                }
                (
                    Some(dml::ScalarValue::String(_)),
                    dml::FieldType::Base(dml::ScalarType::String),
                    dml::FieldArity::Required,
                ) => true,
                (
                    Some(dml::ScalarValue::Int(_)),
                    dml::FieldType::Base(dml::ScalarType::Int),
                    dml::FieldArity::Required,
                ) => true,
                (
                    Some(dml::ScalarValue::Expression(name, return_type, args)),
                    dml::FieldType::Base(dml::ScalarType::Int),
                    dml::FieldArity::Required,
                ) => {
                    let name_eq = name == "autoincrement";
                    let type_eq = return_type == &dml::ScalarType::Int;
                    let args_eq = args.is_empty();

                    name_eq && type_eq && args_eq
                }
                (None, dml::FieldType::Base(dml::ScalarType::Int), dml::FieldArity::Required) => true,
                (None, dml::FieldType::Base(dml::ScalarType::String), dml::FieldArity::Required) => true,
                _ => false,
            };

            if !is_valid {
                return Err(DatamodelError::new_model_validation_error(
                    "Invalid ID field. ID field must be one of: Int @id or Int @id @default(`Integer`|`autoincrement()`) for Int fields or String @id or String @id @default(`cuid()`|`uuid()`|`String`) for String fields.",
                    &model.name,
                    ast_schema.find_field(&model.name, &id_field.name).expect(STATE_ERROR).span));
            }
        }

        Ok(())
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
            for field in model.fields() {
                if !field.is_generated {
                    if let dml::FieldType::Relation(rel) = &field.field_type {
                        // TODO: I am not sure if this check is d'accord with the query engine.
                        let related = datamodel.find_model(&rel.to).unwrap();
                        let related_field = related.related_field(&model.name, &rel.name, &field.name).unwrap();

                        if rel.to_fields.is_empty() && !related_field.is_generated {
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
        }

        Ok(())
    }

    /// Elegantly checks if any relations in the model are ambigious.
    fn validate_relations_not_ambiguous(
        &self,
        ast_schema: &ast::SchemaAst,
        model: &dml::Model,
    ) -> Result<(), DatamodelError> {
        for field_a in model.fields() {
            for field_b in model.fields() {
                if field_a != field_b {
                    if let dml::FieldType::Relation(rel_a) = &field_a.field_type {
                        if let dml::FieldType::Relation(rel_b) = &field_b.field_type {
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
                                for field_c in model.fields() {
                                    if field_a != field_c && field_b != field_c {
                                        if let dml::FieldType::Relation(rel_c) = &field_c.field_type {
                                            if rel_c.to == model.name
                                                && rel_a.name == rel_b.name
                                                && rel_a.name == rel_c.name
                                            {
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
            }
        }

        Ok(())
    }
}
