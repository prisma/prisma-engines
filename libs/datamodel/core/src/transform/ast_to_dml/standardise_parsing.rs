use super::common::*;
use crate::{dml, Field};

/// Helper for standardising a datamodel during parsing.
///
/// This will add relation names, referential actions and M2M references contents
pub fn standardise(schema: &mut dml::Datamodel) {
    set_relation_to_field_to_id_if_missing_for_m2m_relations(schema);
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
