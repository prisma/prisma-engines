use super::common::*;
use crate::error::DatamodelError;
use crate::{ast, common::names::*, dml, error::ErrorCollection, Field, OnDeleteStrategy, UniqueCriteria};

/// Helper for standardsing a datamodel.
///
/// When standardsing, datamodel will be made consistent.
/// Implicit back relation fields, relation names and `to_fields` will be generated.
pub struct Standardiser {}

impl Standardiser {
    /// Creates a new instance, with all builtin directives registered.
    pub fn new() -> Self {
        Standardiser {}
    }

    pub fn standardise(&self, ast_schema: &ast::SchemaAst, schema: &mut dml::Datamodel) -> Result<(), ErrorCollection> {
        self.name_unnamed_relations(schema);

        self.add_missing_back_relations(ast_schema, schema)?;

        // This is intentionally disabled for now, since the generated types would surface in the
        // client schema.
        // self.add_missing_relation_tables(ast_schema, schema)?;
        self.set_relation_to_field_to_id_if_missing(schema);

        Ok(())
    }

    /// For any relations which are missing to_fields, sets them to the @id fields
    /// of the foreign model.
    fn set_relation_to_field_to_id_if_missing(&self, schema: &mut dml::Datamodel) {
        let schema_copy = schema.clone();

        // Iterate and mutate models.
        for model_idx in 0..schema.models.len() {
            let cloned_model = schema.models[model_idx].clone();
            let unique_criteria = self.unique_criteria(&cloned_model);
            let model = &mut schema.models[model_idx];

            for field_index in 0..model.fields.len() {
                let field = &mut model.fields[field_index];

                if let dml::FieldType::Relation(rel) = &mut field.field_type {
                    let related_model = schema_copy.find_model(&rel.to).expect(STATE_ERROR);
                    let related_field = related_model
                        .related_field(model.name.clone().as_str(), &rel.name, &field.name)
                        .unwrap();
                    let related_model_name = &related_model.name;
                    let is_m2m = field.arity.is_list() && related_field.arity.is_list();

                    let related_field_rel = if let dml::FieldType::Relation(rel) = &related_field.field_type {
                        rel
                    } else {
                        panic!(STATE_ERROR)
                    };

                    let embed_here = match (field.arity, related_field.arity) {
                        // many to many
                        (dml::FieldArity::List, dml::FieldArity::List) => true,
                        // one to many
                        (_, dml::FieldArity::List) => true,
                        // many to one
                        (dml::FieldArity::List, _) => false,
                        // one to one
                        (_, _) => match (&model.name, related_model_name) {
                            (x, y) if x < y => true,
                            (x, y) if x > y => false,
                            // SELF RELATIONS
                            _ => field.name < related_field.name,
                        },
                    };

                    let underlying_fields =
                        self.underlying_fields_for_unique_criteria(&unique_criteria, model.name.clone().as_ref());

                    if embed_here {
                        // user input has precedence
                        if rel.to_fields.is_empty() && related_field_rel.to_fields.is_empty() {
                            rel.to_fields = related_model
                                .first_unique_criterion()
                                .iter()
                                .map(|f| f.name.to_owned())
                                .collect();
                        }

                        // user input has precedence
                        if !is_m2m
                            && (rel.fields.is_empty() && related_field_rel.fields.is_empty())
                            && field.is_generated
                        {
                            rel.fields = underlying_fields.iter().map(|f| f.name.clone()).collect();
                            for underlying_field in underlying_fields {
                                model.add_field(underlying_field);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Identifies and adds missing back relations. For 1:1 and 1:N relations.
    /// Explicit n:m relations are not touched, as they already have a back relation field.
    fn add_missing_back_relations(
        &self,
        ast_schema: &ast::SchemaAst,
        schema: &mut dml::Datamodel,
    ) -> Result<(), ErrorCollection> {
        let mut errors = ErrorCollection::new();

        let mut missing_back_relation_fields = Vec::new();
        for model in &schema.models {
            let mut missing_for_model = self.find_missing_back_relation_fields(&model, schema, ast_schema)?;
            missing_back_relation_fields.append(&mut missing_for_model);
        }

        for missing_back_relation_field in missing_back_relation_fields {
            let model = schema
                .find_model(&missing_back_relation_field.model)
                .expect(STATE_ERROR);
            let field_name = &missing_back_relation_field.field.name;

            if model.find_field(&field_name).is_some() {
                let source_model = schema
                    .find_model(&missing_back_relation_field.related_model)
                    .expect(STATE_ERROR);
                let source_field = source_model
                    .find_field(&missing_back_relation_field.related_field)
                    .expect(STATE_ERROR);
                errors.push(field_validation_error(
                                "Automatic related field generation would cause a naming conflict. Please add an explicit opposite relation field.",
                                &source_model,
                                &source_field,
                                &ast_schema,
                            ));
            } else {
                let model_mut = schema
                    .find_model_mut(&missing_back_relation_field.model)
                    .expect(STATE_ERROR);

                model_mut.add_field(missing_back_relation_field.field);

                for underlying_field in missing_back_relation_field.underlying_fields.into_iter() {
                    if !model_mut.has_field(&underlying_field.name) {
                        model_mut.add_field(underlying_field);
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
        schema_ast: &ast::SchemaAst,
    ) -> Result<Vec<AddMissingBackRelationField>, ErrorCollection> {
        let mut errors = ErrorCollection::new();

        let mut result = Vec::new();
        for field in model.fields() {
            if let dml::FieldType::Relation(rel) = &field.field_type {
                let mut back_field_exists = false;

                let related_model = schema.find_model(&rel.to).expect(STATE_ERROR);
                if related_model
                    .related_field(&model.name, &rel.name, &field.name)
                    .is_some()
                {
                    back_field_exists = true;
                }

                if !back_field_exists {
                    if field.arity.is_singular() {
                        let relation_info = dml::RelationInfo {
                            to: model.name.clone(),
                            fields: vec![],
                            to_fields: vec![],
                            name: rel.name.clone(),
                            on_delete: OnDeleteStrategy::None,
                        };
                        let mut back_relation_field =
                            dml::Field::new_generated(&model.name, dml::FieldType::Relation(relation_info));
                        back_relation_field.arity = dml::FieldArity::List;

                        result.push(AddMissingBackRelationField {
                            model: rel.to.clone(),
                            field: back_relation_field,
                            related_model: model.name.to_string(),
                            related_field: field.name.to_string(),
                            underlying_fields: vec![],
                        });
                    } else {
                        let unique_criteria = self.unique_criteria(&model);
                        let unique_criteria_field_names =
                            unique_criteria.fields.iter().map(|f| f.name.to_owned()).collect();

                        let underlying_fields: Vec<Field> = self
                            .underlying_fields_for_unique_criteria(&unique_criteria, &model.name)
                            .into_iter()
                            .map(|f| {
                                // This prevents name conflicts with existing fields on the model
                                let mut f = f;
                                if let Some(existing_field) = related_model.find_field(&f.name) {
                                    if !existing_field.field_type.is_compatible_with(&f.field_type) {
                                        f.name = format!("{}_{}", &f.name, &rel.name);
                                    }
                                }
                                f
                            })
                            .collect();

                        let underlying_field_names = underlying_fields.iter().map(|f| f.name.clone()).collect();
                        let underlying_fields_to_add = underlying_fields
                            .into_iter()
                            .filter_map(|f| {
                                match related_model.find_field(&f.name) {
                                    None => {
                                        // field with name does not exist yet
                                        Some(f)
                                    }
                                    Some(other) if other.field_type.is_compatible_with(&f.field_type) => {
                                        // field with name exists and its type is compatible. We must not add it since we would have a duplicate.
                                        None
                                    }
                                    _ => {
                                        // field with name exists and the type is incompatible.
                                        errors.push(DatamodelError::new_model_validation_error(
                                            &format!(
                                                "Automatic underlying field generation tried to add the field `{}` in model `{}` for the back relation field of `{}` in `{}`. A field with that name exists already and has an incompatible type for the relation. Please add the back relation manually.",
                                                &f.name,
                                                &related_model.name,
                                                &field.name,
                                                &model.name,
                                            ),
                                            &related_model.name,
                                            schema_ast.find_model(&related_model.name)
                                                .expect(ERROR_GEN_STATE_ERROR)
                                                .span,
                                        ));
                                        None
                                    }
                                }
                            })
                            .collect();

                        let relation_info = dml::RelationInfo {
                            to: model.name.clone(),
                            fields: underlying_field_names,
                            to_fields: unique_criteria_field_names,
                            name: rel.name.clone(),
                            on_delete: OnDeleteStrategy::None,
                        };

                        let mut back_relation_field =
                            dml::Field::new_generated(&model.name, dml::FieldType::Relation(relation_info));
                        back_relation_field.arity = dml::FieldArity::Optional;

                        result.push(AddMissingBackRelationField {
                            model: rel.to.clone(),
                            field: back_relation_field,
                            related_model: model.name.to_owned(),
                            related_field: field.name.to_owned(),
                            underlying_fields: underlying_fields_to_add,
                        });
                    };
                }
            }
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(result)
        }
    }

    fn unique_criteria<'a>(&self, model: &'a dml::Model) -> UniqueCriteria<'a> {
        model.loose_unique_criterias().into_iter().next().unwrap()
    }

    fn underlying_fields_for_unique_criteria(
        &self,
        unique_criteria: &dml::UniqueCriteria,
        model_name: &str,
    ) -> Vec<Field> {
        let model_name = model_name.to_owned();
        unique_criteria
            .fields
            .iter()
            .map(|f| {
                let mut underlying_field = Field::new(
                    &format!("{}{}", model_name.camel_case(), f.name.pascal_case()),
                    f.field_type.clone(),
                );
                underlying_field.arity = dml::FieldArity::Optional;
                underlying_field
            })
            .collect()
    }

    fn name_unnamed_relations(&self, datamodel: &mut dml::Datamodel) {
        let unnamed_relations = self.find_unnamed_relations(&datamodel);

        for (model_name, field_name, rel_info) in unnamed_relations {
            // Embedding side.
            let field = datamodel.find_field_mut(&model_name, &field_name).expect(STATE_ERROR);

            if let dml::FieldType::Relation(rel) = &mut field.field_type {
                rel.name = DefaultNames::name_for_unambiguous_relation(&model_name, &rel_info.to);
            } else {
                panic!("Tried to name a non-existing relation.");
            }
        }
    }

    // Returns list of model name, field name and relation info.
    fn find_unnamed_relations(&self, datamodel: &dml::Datamodel) -> Vec<(String, String, dml::RelationInfo)> {
        let mut rels = Vec::new();

        for model in datamodel.models() {
            for field in model.fields() {
                if let dml::FieldType::Relation(rel) = &field.field_type {
                    if rel.name.is_empty() {
                        rels.push((model.name.clone(), field.name.clone(), rel.clone()))
                    }
                }
            }
        }

        rels
    }
}

#[derive(Debug)]
struct AddMissingBackRelationField {
    model: String,
    field: dml::Field,
    related_model: String,
    related_field: String,
    underlying_fields: Vec<dml::Field>,
}
