use query_structure::{Model, ScalarFieldRef};

/// Checks whether a field name resolves to a compound field without materializing the fields.
pub fn is_compound_field(name: &str, model: &Model) -> bool {
    is_compound_id(name, model) || is_index_field(name, model)
}

/// Attempts to resolve a field name to a compound field.
pub fn resolve_compound_field(name: &str, model: &Model) -> Option<Vec<ScalarFieldRef>> {
    resolve_compound_id(name, model).or_else(|| resolve_index_fields(name, model))
}

/// Attempts to match a given name to the (schema) name of a compound id field on the model.
pub fn resolve_compound_id(name: &str, model: &Model) -> Option<Vec<ScalarFieldRef>> {
    model
        .fields()
        .compound_id()
        .and_then(|pk| is_compound_id(name, model).then(|| pk.collect()))
}

/// Attempts to match a given name to the (schema) name of a compound indexes on the model and returns the first match.
pub fn resolve_index_fields(name: &str, model: &Model) -> Option<Vec<ScalarFieldRef>> {
    model
        .unique_indexes()
        .find(|index| index_field_name_matches(name, *index))
        .map(|index| {
            index
                .fields()
                .map(|f| ScalarFieldRef::from((model.dm.clone(), f)))
                .collect()
        })
}

fn is_compound_id(name: &str, model: &Model) -> bool {
    model.fields().compound_id().is_some()
        && model.walker().primary_key().is_some_and(|pk| match pk.name() {
            Some(pk_name) => name == pk_name,
            None => compound_field_name_matches(name, pk.fields().map(|field| field.name())),
        })
}

fn is_index_field(name: &str, model: &Model) -> bool {
    model
        .unique_indexes()
        .any(|index| index_field_name_matches(name, index))
}

fn index_field_name_matches(name: &str, index: psl::parser_database::walkers::IndexWalker<'_>) -> bool {
    match index.name() {
        Some(index_name) => name == index_name,
        None => compound_field_name_matches(name, index.fields().map(|field| field.name())),
    }
}

fn compound_field_name_matches<'a>(mut name: &str, fields: impl Iterator<Item = &'a str>) -> bool {
    let mut first = true;

    for field in fields {
        if first {
            first = false;
        } else if let Some(rest) = name.strip_prefix('_') {
            name = rest;
        } else {
            return false;
        }

        let Some(rest) = name.strip_prefix(field) else {
            return false;
        };
        name = rest;
    }

    !first && name.is_empty()
}
