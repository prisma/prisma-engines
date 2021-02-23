use super::common::*;
use crate::diagnostics::DatamodelError;
use crate::{
    ast, common::NameNormalizer, diagnostics::Diagnostics, dml, Field, OnDeleteStrategy, ScalarField, UniqueCriteria,
};
use itertools::Itertools;
use std::collections::HashMap;

/// Helper for standardsing a datamodel.
///
/// When standardsing, datamodel will be made consistent.
/// Implicit back relation fields, relation names and `references` will be generated.
pub struct StandardiserForFormatting {}
impl StandardiserForFormatting {
    /// Creates a new instance, with all builtin attributes registered.
    pub fn new() -> Self {
        StandardiserForFormatting {}
    }

    pub fn standardise(&self, ast_schema: &ast::SchemaAst, schema: &mut dml::Datamodel) -> Result<(), Diagnostics> {
        self.add_missing_back_relations(ast_schema, schema)?;
        self.set_relation_to_field_to_id_if_missing(ast_schema, schema)?;
        Ok(())
    }

    /// For any relations which are missing references, sets them to the @id fields
    /// of the foreign model.
    /// Also adds missing underlying scalar fields.
    fn set_relation_to_field_to_id_if_missing(
        &self,
        ast_schema: &ast::SchemaAst,
        schema: &mut dml::Datamodel,
    ) -> Result<(), Diagnostics> {
        let mut errors = Diagnostics::new();
        let schema_copy = schema.clone();

        // Iterate and mutate models.
        for model in schema.models_mut() {
            let cloned_model = model.clone();

            let mut fields_to_add = vec![];
            let mut missing_field_names_to_field_names = HashMap::new();
            for field in model.fields_mut() {
                if let Field::RelationField(field) = field {
                    let related_model = schema_copy.find_model(&field.relation_info.to).expect(STATE_ERROR);
                    let related_field = schema_copy.find_related_field_bang(field);
                    let related_model_name = &related_model.name;
                    let rel_info = &mut field.relation_info;
                    let related_field_rel_info = &related_field.relation_info;

                    let embed_here = match (field.arity, related_field.arity) {
                        // many to many
                        (dml::FieldArity::List, dml::FieldArity::List) => false, //handled during parsing
                        // one to many
                        (_, dml::FieldArity::List) => true,
                        // many to one
                        (dml::FieldArity::List, _) => false,
                        // one to one
                        (_, _) => match (&cloned_model.name, related_model_name) {
                            (x, y) if x < y => true,
                            (x, y) if x > y => false,
                            // SELF RELATIONS
                            _ => field.name < related_field.name,
                        },
                    };

                    if embed_here {
                        // user input has precedence
                        if rel_info.references.is_empty() && related_field_rel_info.references.is_empty() {
                            rel_info.references = related_model
                                .first_unique_criterion()
                                .iter()
                                .map(|f| f.name.to_owned())
                                .collect();
                        }

                        // user input has precedence
                        if rel_info.fields.is_empty() && related_field_rel_info.fields.is_empty() {
                            let unique_criteria = self.unique_criteria(&related_model);
                            let underlying_fields = self.underlying_fields_for_unique_criteria(
                                &unique_criteria,
                                &related_model.name,
                                field.arity,
                            );
                            rel_info.fields = underlying_fields.iter().map(|f| f.name.clone()).collect();

                            for underlying_field in underlying_fields {
                                let t = missing_field_names_to_field_names
                                    .entry(underlying_field.clone().name)
                                    .or_insert_with(Vec::new);

                                t.push(field.name.clone());
                                let scalar_field = Field::ScalarField(underlying_field);
                                if !fields_to_add.contains(&scalar_field) {
                                    fields_to_add.push(scalar_field);
                                }
                            }
                        }
                    }
                }
            }
            for field in fields_to_add {
                match missing_field_names_to_field_names.get(field.name()) {
                    Some(field_names) if field_names.len() > 1 => {
                        let model_span = ast_schema
                            .find_model(cloned_model.name.as_str())
                            .expect(ERROR_GEN_STATE_ERROR)
                            .span;

                        let missing_names = field_names
                            .iter()
                            .map(|f| format!("{}Id", f.camel_case()))
                            .collect_vec();

                        errors.push_error(DatamodelError::new_model_validation_error(
                            format!(
                                "Colliding implicit relations. Please add scalar types {}.",
                                missing_names.join(", and ")
                            )
                            .as_str(),
                            cloned_model.name.as_str(),
                            model_span,
                        ));
                    }
                    Some(_) => model.add_field(field),
                    None => {}
                }
            }
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(())
        }
    }

    /// Identifies and adds missing back relations. For 1:1 and 1:N relations.
    /// Explicit n:m relations are not touched, as they already have a back relation field.
    /// Underlying scalar fields are not added here.
    fn add_missing_back_relations(
        &self,
        ast_schema: &ast::SchemaAst,
        schema: &mut dml::Datamodel,
    ) -> Result<(), Diagnostics> {
        let mut errors = Diagnostics::new();

        let mut missing_back_relation_fields = Vec::new();
        for model in schema.models() {
            let mut missing_for_model = self.find_missing_back_relation_fields(&model, schema);
            missing_back_relation_fields.append(&mut missing_for_model);
        }

        for missing_back_relation_field in missing_back_relation_fields {
            let model = schema
                .find_model(&missing_back_relation_field.model)
                .expect(STATE_ERROR);
            let field_name = &missing_back_relation_field.field.name;

            if model.find_relation_field(&field_name).is_some() {
                let source_model = schema
                    .find_model(&missing_back_relation_field.related_model)
                    .expect(STATE_ERROR);
                let source_field = source_model
                    .find_relation_field(&missing_back_relation_field.related_field)
                    .expect(STATE_ERROR);
                errors.push_error(field_validation_error(
                                "Automatic related field generation would cause a naming conflict. Please add an explicit opposite relation field.",
                                &source_model,
                                &Field::RelationField(source_field.clone()),
                                &ast_schema,
                            ));
            } else {
                let model_mut = schema.find_model_mut(&missing_back_relation_field.model);

                model_mut.add_field(Field::RelationField(missing_back_relation_field.field));

                for underlying_field in missing_back_relation_field.underlying_fields.into_iter() {
                    if !model_mut.has_field(&underlying_field.name) {
                        model_mut.add_field(Field::ScalarField(underlying_field));
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

    fn find_missing_back_relation_fields(
        &self,
        model: &dml::Model,
        schema: &dml::Datamodel,
    ) -> Vec<AddMissingBackRelationField> {
        let mut result = Vec::new();
        for field in model.relation_fields() {
            let rel_info = &field.relation_info;
            let related_model = schema.find_model(&rel_info.to).expect(STATE_ERROR);

            if schema.find_related_field(&field).is_none() {
                if field.is_singular() {
                    let relation_info = dml::RelationInfo {
                        to: model.name.clone(),
                        fields: vec![],
                        references: vec![],
                        name: rel_info.name.clone(),
                        on_delete: OnDeleteStrategy::None,
                    };
                    let mut back_relation_field = dml::RelationField::new_generated(&model.name, relation_info);
                    back_relation_field.arity = dml::FieldArity::List;
                    back_relation_field.is_ignored = model.is_ignored;

                    result.push(AddMissingBackRelationField {
                        model: rel_info.to.clone(),
                        field: back_relation_field,
                        related_model: model.name.to_string(),
                        related_field: field.name.to_string(),
                        underlying_fields: vec![],
                    });
                } else {
                    let unique_criteria = self.unique_criteria(&model);
                    let unique_criteria_field_names =
                        unique_criteria.fields.iter().map(|f| f.name.to_owned()).collect();

                    let underlying_fields: Vec<ScalarField> = self
                        .underlying_fields_for_unique_criteria(&unique_criteria, &model.name, dml::FieldArity::Optional)
                        .into_iter()
                        .map(|f| {
                            // This prevents name conflicts with existing fields on the model
                            let mut f = f;
                            if let Some(existing_field) = related_model.find_field(&f.name) {
                                if !existing_field.field_type().is_compatible_with(&f.field_type) {
                                    f.name = format!("{}_{}", &f.name, &rel_info.name);
                                }
                            }
                            f
                        })
                        .collect();

                    let underlying_field_names = underlying_fields.iter().map(|f| f.name.clone()).collect();

                    let mut cont = false;
                    let underlying_fields_to_add = underlying_fields
                        .into_iter()
                        .filter(|f| {
                            match related_model.find_field(&f.name) {
                                None => {
                                    // field with name does not exist yet
                                    true
                                }
                                Some(other) if other.field_type().is_compatible_with(&f.field_type) => {
                                    // field with name exists and its type is compatible. We must not add it since we would have a duplicate.
                                    false
                                }
                                _ => {
                                    // field with name exists and its type is incompatible. We should not add a backrelation at all.
                                    // todo but then we would fail the postvalidation
                                    // maybe we shold not have that one for format since we can't always be right?
                                    cont = true;
                                    false
                                }
                            }
                        })
                        .collect();

                    if cont {
                        continue;
                    }

                    let relation_info = dml::RelationInfo {
                        to: model.name.clone(),
                        fields: underlying_field_names,
                        references: unique_criteria_field_names,
                        name: rel_info.name.clone(),
                        on_delete: OnDeleteStrategy::None,
                    };

                    let back_relation_field = dml::RelationField::new_generated(&model.name, relation_info);
                    result.push(AddMissingBackRelationField {
                        model: rel_info.to.clone(),
                        field: back_relation_field,
                        related_model: model.name.to_owned(),
                        related_field: field.name.to_owned(),
                        underlying_fields: underlying_fields_to_add,
                    });
                };
            }
        }

        result
    }

    fn unique_criteria<'a>(&self, model: &'a dml::Model) -> UniqueCriteria<'a> {
        model.loose_unique_criterias().into_iter().next().unwrap()
    }

    fn underlying_fields_for_unique_criteria(
        &self,
        unique_criteria: &dml::UniqueCriteria,
        model_name: &str,
        field_arity: dml::FieldArity,
    ) -> Vec<ScalarField> {
        let model_name = model_name.to_owned();
        unique_criteria
            .fields
            .iter()
            .map(|f| {
                ScalarField::new(
                    &format!("{}{}", model_name.camel_case(), f.name.pascal_case()),
                    field_arity,
                    f.field_type.clone(),
                )
            })
            .collect()
    }
}

#[derive(Debug)]
struct AddMissingBackRelationField {
    model: String,
    field: dml::RelationField,
    related_model: String,
    related_field: String,
    underlying_fields: Vec<dml::ScalarField>,
}
