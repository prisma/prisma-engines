use super::common::*;
use crate::common::constraint_names::ConstraintNames;
use crate::{common::RelationNames, dml, Datasource, Field, WithDatabaseName};

/// Helper for standardising a datamodel during parsing.
///
/// This will add relation names, referential actions and M2M references contents
pub fn standardise(schema: &mut dml::Datamodel, source: Option<&Datasource>) {
    name_unnamed_relations(schema);
    set_relation_to_field_to_id_if_missing_for_m2m_relations(schema);
    set_referential_arities(schema);
    add_implicit_unique_constraints_for_1_to_1_relations(schema, source);
}

fn add_implicit_unique_constraints_for_1_to_1_relations(schema: &mut dml::Datamodel, source: Option<&Datasource>) {
    let mut modifications = Vec::new();

    for (model_id, model) in schema.models().enumerate() {
        for field in model.fields() {
            match field {
                Field::RelationField(field) if field.is_singular() && !field.relation_info.fields.is_empty() => {
                    if let Some(src) = source {
                        let related_field_is_singular =
                            matches!(schema.find_related_field(field), Some(rf) if rf.1.is_singular());

                        let covered_by_index = model
                            .indices
                            .iter()
                            .any(|index| index.fields == field.relation_info.fields && index.is_unique());

                        let covered_by_pk =
                            matches!( &model.primary_key, Some(pk) if pk.fields == field.relation_info.fields);

                        if related_field_is_singular && !covered_by_pk && !covered_by_index {
                            let column_names: Vec<&str> = field
                                .relation_info
                                .fields
                                .iter()
                                .map(|field_name| {
                                    schema.models[model_id]
                                        .find_field(field_name)
                                        .unwrap()
                                        .final_database_name()
                                })
                                .collect();

                            let index = dml::IndexDefinition {
                                name: None,
                                db_name: Some(ConstraintNames::unique_index_name(
                                    model.final_database_name(),
                                    &column_names,
                                    src.active_connector.as_ref(),
                                )),
                                fields: field.relation_info.fields.clone(),
                                tpe: dml::IndexType::Unique,
                                defined_on_field: field.relation_info.fields.len() == 1,
                            };

                            modifications.push((model_id, index));
                        }
                    }
                }

                _ => (),
            }
        }
    }

    for (model_id, index) in modifications {
        schema.models[model_id].indices.push(index);
    }
}

fn set_referential_arities(schema: &mut dml::Datamodel) {
    let mut modifications = Vec::new();

    for (model_id, model) in schema.models().enumerate() {
        for (field_id, field) in model.fields().enumerate() {
            match field {
                Field::RelationField(field) if field.is_singular() => {
                    let some_required = field
                        .relation_info
                        .fields
                        .iter()
                        .flat_map(|name| model.find_field(name))
                        .any(|field| field.arity().is_required());

                    let arity = if some_required {
                        dml::FieldArity::Required
                    } else {
                        field.arity
                    };

                    modifications.push((model_id, field_id, arity));
                }
                _ => (),
            }
        }
    }

    for (model_id, field_id, arity) in modifications {
        let mut field = schema.models[model_id].fields[field_id]
            .as_relation_field_mut()
            .unwrap();

        field.referential_arity = arity;
    }
}

/// For M2M relations set the references to the @id fields of the foreign model.
fn set_relation_to_field_to_id_if_missing_for_m2m_relations(schema: &mut dml::Datamodel) {
    let schema_copy = schema.clone();

    // Iterate and mutate models.
    for model in schema.models_mut() {
        for field in model.fields_mut() {
            if let Field::RelationField(field) = field {
                if let Some((_rel_field_idx, related_field)) = schema_copy.find_related_field(field) {
                    let related_model = schema_copy.find_model(&field.relation_info.to).expect(STATE_ERROR);
                    let rel_info = &mut field.relation_info;
                    let related_field_rel_info = &related_field.relation_info;

                    if field.arity.is_list()
                        && related_field.arity.is_list()
                        && rel_info.references.is_empty()
                        && related_field_rel_info.references.is_empty()
                    {
                        rel_info.references = related_model
                            .first_unique_criterion()
                            .iter()
                            .map(|f| f.name.to_owned())
                            .collect();
                    }
                }
            }
        }
    }
}

fn name_unnamed_relations(datamodel: &mut dml::Datamodel) {
    let unnamed_relations = find_unnamed_relations(datamodel);

    for (model_name, field_name, rel_info) in unnamed_relations {
        // Embedding side.
        let field = datamodel.find_relation_field_mut(&model_name, &field_name);
        field.relation_info.name = RelationNames::name_for_unambiguous_relation(&model_name, &rel_info.to);
    }
}

// Returns list of model name, field name and relation info.
fn find_unnamed_relations(datamodel: &dml::Datamodel) -> Vec<(String, String, dml::RelationInfo)> {
    let mut rels = Vec::new();

    for model in datamodel.models() {
        for field in model.relation_fields() {
            if field.relation_info.name.is_empty() {
                rels.push((model.name.clone(), field.name.clone(), field.relation_info.clone()))
            }
        }
    }

    rels
}
