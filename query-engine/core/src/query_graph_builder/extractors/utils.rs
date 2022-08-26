use crate::schema_builder;
use prisma_models::{IndexField, ModelRef};

/// Attempts to resolve a field name to a compound field.
pub fn resolve_compound_field(name: &str, model: &ModelRef) -> Option<Vec<IndexField>> {
    resolve_compound_id(name, model).or_else(|| resolve_index_fields(name, model))
}

/// Attempts to match a given name to the (schema) name of a compound id field on the model.
fn resolve_compound_id(name: &str, model: &ModelRef) -> Option<Vec<IndexField>> {
    model.fields().compound_id().and_then(|pk| {
        (name == schema_builder::compound_id_field_name(pk)).then(|| IndexField::from_scalars(pk.fields()))
    })
}

/// Attempts to match a given name to the (schema) name of a compound indexes on the model and returns the first match.
fn resolve_index_fields(name: &str, model: &ModelRef) -> Option<Vec<IndexField>> {
    model
        .unique_indexes()
        .into_iter()
        .find(|index| schema_builder::compound_index_field_name(index) == name)
        .map(|index| index.fields().to_vec())
}

pub fn resolve_composite_index_field(name: &str, model: &ModelRef) -> Option<Vec<IndexField>> {
    // model.indexes().into_iter().find(|i| i.fields().len() == 1 && i.f)
    None
}
