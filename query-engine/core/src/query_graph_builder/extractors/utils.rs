use query_structure::{Model, ScalarFieldRef};

/// Attempts to resolve a field name to a compound field.
pub fn resolve_compound_field(name: &str, model: &Model) -> Option<Vec<ScalarFieldRef>> {
    resolve_compound_id(name, model).or_else(|| resolve_index_fields(name, model))
}

/// Attempts to match a given name to the (schema) name of a compound id field on the model.
pub fn resolve_compound_id(name: &str, model: &Model) -> Option<Vec<ScalarFieldRef>> {
    model.fields().compound_id().and_then(|pk| {
        (name == schema::compound_id_field_name(model.walker().primary_key().unwrap())).then(|| pk.collect())
    })
}

/// Attempts to match a given name to the (schema) name of a compound indexes on the model and returns the first match.
pub fn resolve_index_fields(name: &str, model: &Model) -> Option<Vec<ScalarFieldRef>> {
    model
        .unique_indexes()
        .find(|index| schema::compound_index_field_name(*index) == name)
        .map(|index| {
            index
                .fields()
                .map(|f| ScalarFieldRef::from((model.dm.clone(), f)))
                .collect()
        })
}
