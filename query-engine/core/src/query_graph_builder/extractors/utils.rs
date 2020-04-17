use crate::schema_builder;
use prisma_models::{ModelRef, ScalarFieldRef};

/// Attempts to resolve a field name to a compound field.
pub fn resolve_compound_field(name: &str, model: &ModelRef) -> Option<Vec<ScalarFieldRef>> {
    resolve_compound_id(name, model).or_else(|| resolve_index_fields(name, model))
}

/// Attempts to match a given name to the (schema) name of a compound id field on the model.
pub fn resolve_compound_id(name: &str, model: &ModelRef) -> Option<Vec<ScalarFieldRef>> {
    model.fields().id().and_then(|fields| {
        let names = fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>();

        if name == schema_builder::compound_id_field_name(&names) {
            Some(fields)
        } else {
            None
        }
    })
}

/// Attempts to match a given name to the (schema) name of a compound indexes on the model and returns the first match.
pub fn resolve_index_fields(name: &str, model: &ModelRef) -> Option<Vec<ScalarFieldRef>> {
    model
        .unique_indexes()
        .into_iter()
        .find(|index| &schema_builder::compound_index_field_name(index) == name)
        .map(|index| index.fields())
}
