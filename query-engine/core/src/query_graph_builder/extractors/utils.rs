use crate::schema_builder;
use prisma_models::{ModelRef, ScalarFieldRef};

pub fn resolve_compound_field(name: &str, model: &ModelRef) -> Option<Vec<ScalarFieldRef>> {
    resolve_compound_id(name, model).or_else(|| resolve_index_fields(name, model))
}

/// Attempts to match a given name to the (schema) name of a compound id field on the model.
pub fn resolve_compound_id(name: &str, model: &ModelRef) -> Option<Vec<ScalarFieldRef>> {
    model.fields.id()
}

/// Attempts to match a given name to the (schema) name of a compound indexes on the model and returns the first match.
pub fn resolve_index_fields(name: &str, model: &ModelRef) -> Option<Vec<ScalarFieldRef>> {
    model
        .unique_indexes()
        .into_iter()
        .find(|index| &schema_builder::compound_field_name(index) == name)
        .map(|index| index.fields())
}
